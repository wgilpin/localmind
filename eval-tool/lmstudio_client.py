#!/usr/bin/env python3
"""
LM Studio Client Module
Provides interface to LM Studio via OpenAI-compatible API
"""

import requests
import json
from typing import List, Dict, Optional
from openai import OpenAI


class LMStudioClient:
    """Client for LM Studio API"""

    def __init__(self, base_url: str = "http://localhost:1234", api_key: str = "lm-studio"):
        """
        Initialize LM Studio client

        Args:
            base_url: LM Studio server URL
            api_key: API key (LM Studio uses dummy key)
        """
        self.base_url = base_url
        self.client = OpenAI(
            base_url=f"{base_url}/v1",
            api_key=api_key
        )

    def list_models(self) -> List[Dict]:
        """List available models"""
        try:
            models = self.client.models.list()
            return [{"id": model.id, "object": model.object} for model in models.data]
        except Exception as e:
            print(f"Error listing models: {e}")
            return []

    def chat(self, model: str, messages: List[Dict], **kwargs) -> Dict:
        """
        Chat completion

        Args:
            model: Model name/ID
            messages: List of message dictionaries
            **kwargs: Additional options (temperature, max_tokens, etc.)

        Returns:
            Response dictionary
        """
        try:
            # Extract options
            temperature = kwargs.get('temperature', 0.7)
            max_tokens = kwargs.get('max_tokens', kwargs.get('num_predict', 1000))
            top_p = kwargs.get('top_p', 0.9)

            response = self.client.chat.completions.create(
                model=model,
                messages=messages,
                temperature=temperature,
                max_tokens=max_tokens,
                top_p=top_p
            )

            # Convert to Ollama-compatible format
            return {
                'message': {
                    'content': response.choices[0].message.content
                }
            }

        except Exception as e:
            raise Exception(f"LM Studio API error: {e}")

    def generate(self, model: str, prompt: str, **kwargs) -> Dict:
        """
        Text generation (for compatibility)

        Args:
            model: Model name/ID
            prompt: Input prompt
            **kwargs: Additional options

        Returns:
            Response dictionary
        """
        messages = [{"role": "user", "content": prompt}]
        return self.chat(model, messages, **kwargs)

    def check_server(self) -> bool:
        """Check if LM Studio server is running"""
        try:
            response = requests.get(f"{self.base_url}/v1/models", timeout=5)
            return response.status_code == 200
        except:
            return False

    def get_server_info(self) -> Dict:
        """Get server information"""
        try:
            models = self.list_models()
            return {
                "server": "LM Studio",
                "base_url": self.base_url,
                "available_models": len(models),
                "models": models
            }
        except Exception as e:
            return {"error": str(e)}


def main():
    """Test LM Studio client"""
    client = LMStudioClient()

    print("Testing LM Studio connection...")

    if not client.check_server():
        print("❌ LM Studio server not running at http://localhost:1234")
        print("Please start LM Studio and load a model")
        return

    print("✅ LM Studio server is running")

    # List models
    models = client.list_models()
    print(f"Available models: {len(models)}")
    for model in models:
        print(f"  - {model['id']}")

    if models:
        # Test chat
        model_id = models[0]['id']
        print(f"\nTesting chat with {model_id}...")

        try:
            response = client.chat(
                model=model_id,
                messages=[{"role": "user", "content": "Hello, respond with just 'Hi!'"}],
                max_tokens=10
            )
            print(f"Response: {response['message']['content']}")
        except Exception as e:
            print(f"Error: {e}")


if __name__ == '__main__':
    main()