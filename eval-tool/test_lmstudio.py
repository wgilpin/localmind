#!/usr/bin/env python3
"""
Test LM Studio connection and models
"""

from lmstudio_client import LMStudioClient

def main():
    print("Testing LM Studio connection...")

    client = LMStudioClient()

    # Check server
    if not client.check_server():
        print("[ERROR] LM Studio server not running at http://localhost:1234")
        print("\nTo fix:")
        print("1. Start LM Studio")
        print("2. Go to Local Server tab")
        print("3. Start the server")
        print("4. Load a model (e.g., qwen2.5-3b-instruct)")
        return

    print("[OK] LM Studio server is running")

    # List models
    models = client.list_models()
    if not models:
        print("[ERROR] No models loaded")
        print("\nTo fix:")
        print("1. In LM Studio, go to Local Server tab")
        print("2. Select a model and click 'Load Model'")
        return

    print(f"[OK] Found {len(models)} loaded model(s):")
    for model in models:
        print(f"  - {model['id']}")

    # Test chat
    model_id = models[0]['id']
    print(f"\n[TEST] Testing chat with {model_id}...")

    try:
        response = client.chat(
            model=model_id,
            messages=[{"role": "user", "content": "Say 'Hello from LM Studio!'"}],
            max_tokens=20
        )
        print(f"[OK] Response: {response['message']['content']}")
        print("\n[SUCCESS] LM Studio is ready for the eval-tool!")

    except Exception as e:
        print(f"[ERROR] Error testing chat: {e}")

if __name__ == '__main__':
    main()