"""
Whisper ASR sidecar - mimics whisper.cpp's HTTP API.
Uses Optimum + ONNX Runtime + Xenova/whisper-medium model.

Endpoints:
- GET  /health        -> 200 OK
- POST /v1/audio/transcriptions  -> OpenAI-compatible transcription
"""
import io
import os
import sys
import time
import wave
import logging
import threading
from pathlib import Path

import numpy as np
from fastapi import FastAPI, File, Form, HTTPException, UploadFile
from fastapi.responses import JSONResponse, PlainTextResponse
import uvicorn

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    stream=sys.stdout,
)
log = logging.getLogger("whisper-asr")

PORT = int(os.environ.get("ASR_PORT", "8178"))
MODEL_DIR = Path(os.environ.get("ASR_MODEL_DIR",
    str(Path(__file__).parent / "whisper-model")))


class WhisperService:
    def __init__(self, model_dir: Path):
        self.model_dir = model_dir
        self.model = None
        self.processor = None
        self.lock = threading.Lock()
        self.loaded = False

    def load(self):
        if self.loaded:
            return
        with self.lock:
            if self.loaded:
                return
            log.info(f"Loading Whisper ONNX model from {self.model_dir}")
            from optimum.onnxruntime import ORTModelForSpeechSeq2Seq
            from transformers import WhisperProcessor
            import onnxruntime as ort

            # Configure session options to avoid graph optimization bug
            sess_opts = ort.SessionOptions()
            sess_opts.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_BASIC
            sess_opts.intra_op_num_threads = max(1, os.cpu_count() // 2)

            self.model = ORTModelForSpeechSeq2Seq.from_pretrained(
                str(self.model_dir),
                use_cache=False,
                session_options=sess_opts,
                # The ONNX export uses _fp16 suffix; without these hints
                # optimum fails with "Could not find any ONNX files".
                encoder_file_name="encoder_model_fp16.onnx",
                decoder_file_name="decoder_model_merged_fp16.onnx",
            )
            self.processor = WhisperProcessor.from_pretrained(str(self.model_dir))
            self.loaded = True
            log.info("Models loaded.")

    def transcribe_waveform(self, wav_bytes: bytes) -> str:
        self.load()

        # Decode WAV
        with io.BytesIO(wav_bytes) as buf:
            with wave.open(buf, "rb") as wf:
                n = wf.getnframes()
                sr = wf.getframerate()
                sw = wf.getsampwidth()
                ch = wf.getnchannels()
                raw = wf.readframes(n)
                if sw == 2:
                    pcm = np.frombuffer(raw, dtype=np.int16).astype(np.float32) / 32768.0
                elif sw == 4:
                    pcm = np.frombuffer(raw, dtype=np.int32).astype(np.float32) / 2147483648.0
                elif sw == 1:
                    pcm = (np.frombuffer(raw, dtype=np.uint8).astype(np.float32) - 128.0) / 128.0
                else:
                    raise ValueError(f"unsupported sample width: {sw}")
                if ch > 1:
                    pcm = pcm.reshape(-1, ch).mean(axis=1)

        if sr != 16000:
            from scipy.signal import resample_poly
            from math import gcd
            g = gcd(sr, 16000)
            pcm = resample_poly(pcm, 16000 // g, sr // g).astype(np.float32)

        # Process
        input_features = self.processor(
            pcm, sampling_rate=16000, return_tensors="pt"
        ).input_features

        # Generate
        from transformers import GenerationConfig
        gen_cfg = GenerationConfig.from_pretrained(str(self.model_dir))
        gen_cfg.max_new_tokens = 200
        gen_cfg.num_beams = 1
        gen_cfg.use_cache = False
        forced_decoder_ids = None

        outputs = self.model.generate(
            input_features,
            generation_config=gen_cfg,
            forced_decoder_ids=forced_decoder_ids,
        )
        text = self.processor.batch_decode(outputs, skip_special_tokens=True)[0]
        return text.strip()


service = WhisperService(MODEL_DIR)
app = FastAPI(title="Whisper ASR Sidecar")


@app.get("/health")
def health():
    return PlainTextResponse("OK")


@app.post("/v1/audio/transcriptions")
async def transcriptions(
    file: UploadFile = File(...),
    model: str = Form("whisper-1"),
    language: str = Form("auto"),
    response_format: str = Form("text"),
):
    if file is None:
        raise HTTPException(400, "no file provided")
    raw = await file.read()
    log.info(f"got {len(raw)} bytes, lang={language}")
    t0 = time.time()
    try:
        text = service.transcribe_waveform(raw)
    except Exception as e:
        log.exception("transcribe failed")
        raise HTTPException(500, str(e))
    log.info(f"transcribed in {time.time()-t0:.2f}s: {text[:120]!r}")
    if response_format == "text":
        return PlainTextResponse(text)
    return JSONResponse({"text": text})


if __name__ == "__main__":
    log.info(f"Starting Whisper ASR sidecar on port {PORT}, model dir {MODEL_DIR}")
    uvicorn.run(app, host="127.0.0.1", port=PORT, log_level="warning", workers=1)