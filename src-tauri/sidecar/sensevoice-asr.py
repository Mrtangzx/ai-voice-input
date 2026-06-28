"""
SenseVoice ASR sidecar - Chinese-optimized speech recognition.
Uses ModelScope's funasr + SenseVoiceSmall model.
Drop-in replacement for whisper-asr.py on the same port (8178).

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

from fastapi import FastAPI, File, Form, HTTPException, UploadFile
from fastapi.responses import JSONResponse, PlainTextResponse
import uvicorn
import numpy as np

# Force UTF-8 stdout so Chinese transcriptions don't blow up Windows consoles.
try:
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
except Exception:
    pass

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    stream=sys.stdout,
)
log = logging.getLogger("sensevoice-asr")

PORT = int(os.environ.get("ASR_PORT", "8178"))
MODEL_DIR = Path(os.environ.get(
    "ASR_MODEL_DIR",
    str(Path(__file__).parent / "modelscope" / "iic" / "SenseVoiceSmall"),
))
LANGUAGE = os.environ.get("ASR_LANGUAGE", "auto")  # auto | zh | en | ja | ko | yue


class SenseVoiceService:
    def __init__(self, model_dir: Path, language: str = "auto"):
        self.model_dir = model_dir
        self.language = language
        self.model = None
        self.lock = threading.Lock()
        self.loaded = False

    def load(self):
        if self.loaded:
            return
        with self.lock:
            if self.loaded:
                return
            log.info(f"loading SenseVoice model from {self.model_dir}")
            t0 = time.time()
            from funasr import AutoModel
            self.model = AutoModel(
                model=str(self.model_dir),
                trust_remote_code=False,
                device="cpu",
                disable_update=True,
            )
            self.loaded = True
            log.info(f"model loaded in {time.time()-t0:.1f}s")

    def transcribe_waveform(self, wav_bytes: bytes) -> str:
        self.load()
        # SenseVoice expects 16kHz mono PCM via file path, so write to a temp
        # file (or reuse a buffer + soundfile). Stdlib wave avoids soundfile's
        # C library dependency that often fails to install on Windows.
        with io.BytesIO(wav_bytes) as buf:
            with wave.open(buf, "rb") as wf:
                sr = wf.getframerate()
                sw = wf.getsampwidth()
                ch = wf.getnchannels()
                n = wf.getnframes()
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

        # funasr's AutoModel.generate can also accept a numpy array directly.
        # We pass the PCM in-memory so we never touch the filesystem.
        lang = None if self.language == "auto" else self.language
        kwargs = {"input": pcm, "use_itn": True, "sampling_rate": 16000}
        if lang:
            kwargs["language"] = lang
        result = self.model.generate(**kwargs)
        # funasr returns a list of dicts: [{'key': ..., 'text': '...'}]
        if not result:
            return ""
        return result[0].get("text", "").strip()


service = SenseVoiceService(MODEL_DIR, LANGUAGE)
app = FastAPI(title="SenseVoice ASR Sidecar")


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
    except Exception:
        log.exception("transcribe failed")
        raise HTTPException(500, "transcribe failed")
    elapsed = time.time() - t0
    log.info(f"transcribed in {elapsed:.2f}s: {text[:120]!r}")
    if response_format == "text":
        return PlainTextResponse(text)
    return JSONResponse({"text": text})


if __name__ == "__main__":
    log.info(f"starting SenseVoice ASR sidecar on port {PORT}, model dir {MODEL_DIR}")
    uvicorn.run(app, host="127.0.0.1", port=PORT, log_level="warning", workers=1)
