#!/usr/bin/env python3
"""
LocalMind Embedding Server

A FastAPI server that provides local embedding generation using sentence-transformers
and the google/embeddinggemma-300M model.

This server is designed to run locally alongside the LocalMind RAG application,
providing embeddings without requiring external LLM services.
"""

import logging
import os
import sys
from enum import Enum

try:
    from typing import TypedDict  # Python 3.12+
except ImportError:
    from typing_extensions import TypedDict  # Python < 3.12

import torch
from fastapi import FastAPI, HTTPException, status
from fastapi.responses import JSONResponse
from sentence_transformers import SentenceTransformer

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)],
)
logger = logging.getLogger(__name__)


# Type definitions
class EmbeddingRequest(TypedDict):
    """Request payload for embedding generation."""

    text: str


class EmbeddingResponse(TypedDict):
    """Response payload containing generated embedding."""

    embedding: list[float]
    model: str
    dimension: int


class HealthResponse(TypedDict):
    """Health check response."""

    status: str
    model_loaded: bool


class ErrorResponse(TypedDict):
    """Error response payload."""

    error: str
    detail: str | None


class ServerState(Enum):
    """Server loading state."""

    STARTING = "starting"
    LOADING = "loading"
    READY = "ready"
    ERROR = "error"


# Global state
app = FastAPI(
    title="LocalMind Embedding Server",
    description="Local embedding generation using google/embeddinggemma-300M",
    version="0.1.0",
)

model: SentenceTransformer | None = None
server_state: ServerState = ServerState.STARTING
state_error: str | None = None

# Constants
MODEL_NAME = "google/embeddinggemma-300M"
EXPECTED_DIMENSION = 768
MAX_TEXT_LENGTH = 2000


def load_model() -> SentenceTransformer:
    """
    Load the embeddinggemma-300M model from Hugging Face.

    Returns:
        Loaded SentenceTransformer model

    Raises:
        RuntimeError: If model loading fails
        MemoryError: If insufficient memory for model loading
    """
    global server_state, state_error

    try:
        server_state = ServerState.LOADING
        logger.info(f"Loading model: {MODEL_NAME}")

        # Determine device
        device = "cuda" if torch.cuda.is_available() else "cpu"
        logger.info(f"Using device: {device}")

        if device == "cuda":
            logger.info(f"GPU: {torch.cuda.get_device_name(0)}")
            logger.info(f"CUDA Version: {torch.version.cuda}")

        # Load model
        loaded_model = SentenceTransformer(MODEL_NAME, device=device)

        # Validate model output dimensions
        test_embedding = loaded_model.encode(["test"])[0]
        actual_dim = len(test_embedding)

        if actual_dim != EXPECTED_DIMENSION:
            raise RuntimeError(
                f"Model dimension mismatch: expected {EXPECTED_DIMENSION}, got {actual_dim}"
            )

        # Log model info
        total_params = sum(p.numel() for p in loaded_model.parameters())
        logger.info("Model loaded successfully")
        logger.info(f"Total parameters: {total_params:,}")
        logger.info(f"Embedding dimension: {actual_dim}")
        logger.info(f"Device: {loaded_model.device}")

        server_state = ServerState.READY
        return loaded_model

    except MemoryError as e:
        error_msg = (
            f"Out of memory while loading model: {e}. "
            "Try closing other applications or use a machine with more RAM."
        )
        logger.error(error_msg)
        server_state = ServerState.ERROR
        state_error = error_msg
        raise MemoryError(error_msg) from e

    except Exception as e:
        error_msg = f"Failed to load model: {e}"
        logger.error(error_msg)
        server_state = ServerState.ERROR
        state_error = error_msg

        # Check if authentication error
        if "401" in str(e) or "authentication" in str(e).lower():
            logger.error(
                "Authentication error. Please authenticate with Hugging Face:\n"
                "  from huggingface_hub import login\n"
                "  login()"
            )

        raise RuntimeError(error_msg) from e


@app.on_event("startup")
async def startup_event() -> None:
    """Initialize model on server startup."""
    global model

    try:
        logger.info("Starting LocalMind Embedding Server...")
        model = load_model()
        logger.info("Server ready to accept requests")
    except Exception as e:
        logger.error(f"Startup failed: {e}")
        # Server will continue running but will return 503 for embed requests


@app.get("/health", response_model=dict)
async def health_check() -> HealthResponse:
    """
    Health check endpoint.

    Returns:
        HealthResponse with server status and model loading state
    """
    return HealthResponse(
        status=server_state.value,
        model_loaded=(model is not None and server_state == ServerState.READY),
    )


@app.post("/embed", response_model=dict)
async def generate_embedding(request: EmbeddingRequest) -> EmbeddingResponse:
    """
    Generate embedding for input text.

    Args:
        request: EmbeddingRequest containing text to embed

    Returns:
        EmbeddingResponse with generated embedding vector

    Raises:
        HTTPException: If model not loaded (503), validation fails (400),
                      or generation fails (500)
    """
    global model

    # Check if model is still loading
    if server_state == ServerState.LOADING:
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail="Model is still loading, please retry",
            headers={"Retry-After": "5"},
        )

    # Check if model failed to load
    if server_state == ServerState.ERROR or model is None:
        error_detail = state_error or "Model failed to load"
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail=error_detail,
        )

    # Validate request
    text = request.get("text", "").strip()

    if not text:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Empty text provided",
        )

    if len(text) > MAX_TEXT_LENGTH:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Text too long (max {MAX_TEXT_LENGTH} characters)",
        )

    # Generate embedding
    try:
        logger.debug(f"Generating embedding for text: {text[:50]}...")

        embedding_array = model.encode([text])[0]
        embedding_list = embedding_array.tolist()

        # Validate dimension
        if len(embedding_list) != EXPECTED_DIMENSION:
            logger.error(
                f"Dimension mismatch: expected {EXPECTED_DIMENSION}, got {len(embedding_list)}"
            )
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Embedding dimension validation failed",
            )

        logger.debug(f"Successfully generated {len(embedding_list)}-dim embedding")

        return EmbeddingResponse(
            embedding=embedding_list,
            model=MODEL_NAME,
            dimension=len(embedding_list),
        )

    except HTTPException:
        raise
    except MemoryError as e:
        logger.error(f"Out of memory during embedding generation: {e}")
        raise HTTPException(
            status_code=status.HTTP_507_INSUFFICIENT_STORAGE,
            detail="Out of memory. Try with shorter text or restart the server.",
        )
    except Exception as e:
        logger.error(f"Embedding generation failed: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Embedding generation failed: {str(e)}",
        )


@app.exception_handler(Exception)
async def global_exception_handler(request: object, exc: Exception) -> JSONResponse:
    """
    Global exception handler for unhandled errors.

    Args:
        request: The request that caused the exception
        exc: The exception that was raised

    Returns:
        JSONResponse with error details
    """
    logger.error(f"Unhandled exception: {exc}", exc_info=True)
    return JSONResponse(
        status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
        content=ErrorResponse(
            error="Internal server error",
            detail=str(exc),
        ),
    )


if __name__ == "__main__":
    import uvicorn

    port = int(os.environ.get("EMBEDDING_SERVER_PORT", 8000))

    logger.info(f"Starting server on port {port}")
    logger.info(f"Model: {MODEL_NAME}")
    logger.info(f"Expected embedding dimension: {EXPECTED_DIMENSION}")

    uvicorn.run(
        app,
        host="0.0.0.0",
        port=port,
        log_level="info",
    )
