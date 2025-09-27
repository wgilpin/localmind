import requests
import json
from typing import List, Union
import time

class OllamaEmbedding:
    """Ollama embedding client that mimics SentenceTransformer interface"""

    def __init__(self, model_name: str, base_url: str = "http://localhost:11434"):
        self.model_name = model_name
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()

        # Test connection and model availability
        self._test_connection()

    def _test_connection(self):
        """Test if Ollama is running and model is available"""
        try:
            # Check if Ollama is running
            response = self.session.get(f"{self.base_url}/api/tags", timeout=5)
            response.raise_for_status()

            # Check if our model is available
            models = response.json().get('models', [])
            model_names = [model['name'] for model in models]

            if self.model_name not in model_names:
                print(f"Warning: Model '{self.model_name}' not found in Ollama.")
                print(f"Available models: {model_names}")
                print(f"You may need to run: ollama pull {self.model_name}")
            else:
                print(f"Ollama connection successful. Using model: {self.model_name}")

        except requests.RequestException as e:
            raise ConnectionError(f"Cannot connect to Ollama at {self.base_url}. "
                                f"Make sure Ollama is running. Error: {e}")

    def encode(self, texts: Union[str, List[str]], **kwargs) -> Union[List[float], List[List[float]]]:
        """
        Generate embeddings for text(s).
        Mimics SentenceTransformer.encode() interface.

        Args:
            texts: Single string or list of strings to encode
            **kwargs: Additional arguments (ignored for compatibility)

        Returns:
            Single embedding (for single string) or list of embeddings (for list of strings)
        """
        if isinstance(texts, str):
            # Single text
            embedding = self._get_embedding(texts)
            return embedding
        else:
            # List of texts
            embeddings = []
            for text in texts:
                embedding = self._get_embedding(text)
                embeddings.append(embedding)
            return embeddings

    def _get_embedding(self, text: str, max_retries: int = 3) -> List[float]:
        """Get embedding for a single text with retry logic"""
        for attempt in range(max_retries):
            try:
                payload = {
                    "model": self.model_name,
                    "prompt": text
                }

                response = self.session.post(
                    f"{self.base_url}/api/embeddings",
                    json=payload,
                    timeout=30
                )
                response.raise_for_status()

                result = response.json()

                if 'embedding' not in result:
                    raise ValueError(f"No embedding in response: {result}")

                return result['embedding']

            except requests.RequestException as e:
                if attempt == max_retries - 1:
                    raise RuntimeError(f"Failed to get embedding after {max_retries} attempts: {e}")

                print(f"Attempt {attempt + 1} failed, retrying in 1 second...")
                time.sleep(1)

        raise RuntimeError(f"Failed to get embedding after {max_retries} attempts")

    def __repr__(self):
        return f"OllamaEmbedding(model='{self.model_name}', base_url='{self.base_url}')"