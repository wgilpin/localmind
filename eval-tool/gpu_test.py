#!/usr/bin/env python3
"""
GPU Usage Test for Ollama Models
Tests different models and configurations to verify GPU acceleration
"""

import subprocess
import json
import time
import psutil
import click
from pathlib import Path
import os

def run_command(cmd):
    """Run a shell command and return output"""
    try:
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        return result.stdout.strip()
    except Exception as e:
        return str(e)

def check_gpu_status():
    """Check current GPU status using nvidia-smi"""
    print("\n" + "="*60)
    print("GPU STATUS CHECK")
    print("="*60)
    
    # Check nvidia-smi
    gpu_info = run_command("nvidia-smi --query-gpu=name,memory.total,memory.free,memory.used,utilization.gpu --format=csv,noheader")
    if gpu_info:
        parts = gpu_info.split(", ")
        if len(parts) >= 5:
            print(f"GPU Name: {parts[0]}")
            print(f"Total Memory: {parts[1]}")
            print(f"Free Memory: {parts[2]}")
            print(f"Used Memory: {parts[3]}")
            print(f"GPU Utilization: {parts[4]}")
    
    # Check if Ollama is using GPU
    print("\n" + "-"*40)
    print("Checking Ollama GPU Usage:")
    
    # Look for ollama processes
    ollama_procs = []
    for proc in psutil.process_iter(['pid', 'name', 'cmdline']):
        try:
            if 'ollama' in proc.info['name'].lower():
                ollama_procs.append(proc)
        except:
            pass
    
    if ollama_procs:
        print(f"Found {len(ollama_procs)} Ollama process(es)")
        # Check GPU processes
        gpu_procs = run_command("nvidia-smi --query-compute-apps=pid,process_name,used_memory --format=csv,noheader")
        if gpu_procs:
            print("GPU Processes:")
            print(gpu_procs)
        else:
            print("No processes currently using GPU")
    else:
        print("No Ollama processes running")
    
    return gpu_info

def test_model_gpu(model_name, prompt="What is 2+2?", max_tokens=50):
    """Test a specific model and monitor GPU usage"""
    print(f"\n" + "-"*40)
    print(f"Testing model: {model_name}")
    print("-"*40)
    
    # Check initial GPU state
    initial_gpu = run_command("nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits")
    print(f"Initial GPU memory: {initial_gpu} MB")
    
    # Run the model
    start_time = time.time()
    
    # Use ollama run with a simple prompt
    cmd = f'echo "{prompt}" | ollama run {model_name} --verbose'
    print(f"Running: {cmd}")
    
    # Start the process and monitor GPU
    process = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    
    # Monitor GPU usage while running
    max_gpu_usage = 0
    while process.poll() is None:
        current_gpu = run_command("nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits")
        try:
            current_usage = int(current_gpu)
            max_gpu_usage = max(max_gpu_usage, current_usage)
        except:
            pass
        time.sleep(0.1)
    
    stdout, stderr = process.communicate()
    end_time = time.time()
    
    # Check final GPU state
    final_gpu = run_command("nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits")
    
    print(f"Response time: {end_time - start_time:.2f} seconds")
    print(f"Max GPU memory during run: {max_gpu_usage} MB")
    print(f"Final GPU memory: {final_gpu} MB")
    
    if stderr and "CPU" in stderr:
        print("⚠️  Model appears to be running on CPU")
    elif max_gpu_usage > int(initial_gpu) + 100:  # If GPU usage increased by more than 100MB
        print("✓ Model appears to be using GPU")
    else:
        print("? Unclear if model is using GPU (minimal memory change)")
    
    if stderr:
        print(f"Stderr output: {stderr[:200]}")
    
    return max_gpu_usage - int(initial_gpu)

def check_ollama_config():
    """Check Ollama configuration for GPU settings"""
    print("\n" + "="*60)
    print("OLLAMA CONFIGURATION")
    print("="*60)
    
    # Check environment variables
    env_vars = ['OLLAMA_NUM_GPU', 'CUDA_VISIBLE_DEVICES', 'OLLAMA_GPU_LAYERS', 'OLLAMA_HOST']
    print("Environment Variables:")
    for var in env_vars:
        value = os.environ.get(var, "Not set")
        print(f"  {var}: {value}")
    
    # Check Ollama version
    version = run_command("ollama --version")
    print(f"\nOllama Version: {version}")
    
    # List available models
    print("\nAvailable Models:")
    models = run_command("ollama list")
    print(models)

def set_gpu_environment():
    """Set environment variables to force GPU usage"""
    print("\n" + "="*60)
    print("SETTING GPU ENVIRONMENT")
    print("="*60)
    
    # Set environment variables for GPU
    os.environ['CUDA_VISIBLE_DEVICES'] = '0'
    os.environ['OLLAMA_NUM_GPU'] = '999'  # Use all layers on GPU
    
    print("Set environment variables:")
    print(f"  CUDA_VISIBLE_DEVICES=0")
    print(f"  OLLAMA_NUM_GPU=999")
    
    # Create a batch script for Windows to set these permanently
    batch_content = """@echo off
REM Set Ollama to use GPU
set CUDA_VISIBLE_DEVICES=0
set OLLAMA_NUM_GPU=999
echo GPU environment variables set for Ollama
"""
    
    batch_path = Path("set_gpu_env.bat")
    batch_path.write_text(batch_content)
    print(f"\nCreated {batch_path} - run this before starting Ollama to ensure GPU usage")

@click.command()
@click.option('--model', default='granite3.3:2b', help='Model to test')
@click.option('--prompt', default='What is the capital of France?', help='Test prompt')
@click.option('--set-env', is_flag=True, help='Set GPU environment variables')
@click.option('--test-all', is_flag=True, help='Test all available models')
def main(model, prompt, set_env, test_all):
    """Test Ollama GPU usage and provide diagnostics"""
    
    if set_env:
        set_gpu_environment()
    
    # Check initial status
    check_ollama_config()
    check_gpu_status()
    
    if test_all:
        # Test all small models
        models_to_test = ['granite3.3:2b', 'qwen3:0.6b', 'gemma3:1b']
        print("\n" + "="*60)
        print("TESTING MULTIPLE MODELS")
        print("="*60)
        
        results = {}
        for model_name in models_to_test:
            try:
                gpu_increase = test_model_gpu(model_name, prompt)
                results[model_name] = gpu_increase
            except Exception as e:
                print(f"Error testing {model_name}: {e}")
                results[model_name] = "Error"
        
        print("\n" + "="*60)
        print("TEST RESULTS SUMMARY")
        print("="*60)
        for model_name, gpu_increase in results.items():
            if isinstance(gpu_increase, int):
                if gpu_increase > 100:
                    status = "✓ Using GPU"
                else:
                    status = "✗ Likely CPU"
                print(f"{model_name}: {status} (GPU memory increase: {gpu_increase} MB)")
            else:
                print(f"{model_name}: {gpu_increase}")
    else:
        # Test single model
        print("\n" + "="*60)
        print(f"TESTING MODEL: {model}")
        print("="*60)
        test_model_gpu(model, prompt)
    
    # Final GPU check
    print("\n" + "="*60)
    print("FINAL GPU STATUS")
    print("="*60)
    check_gpu_status()
    
    print("\n" + "="*60)
    print("RECOMMENDATIONS")
    print("="*60)
    print("1. If models are using CPU, try:")
    print("   - Run: set OLLAMA_NUM_GPU=999")
    print("   - Run: set CUDA_VISIBLE_DEVICES=0")
    print("   - Restart Ollama service: ollama serve")
    print("2. For specific layer control, use: ollama run <model> --gpu-layers 999")
    print("3. Check if model fits in VRAM (24GB available)")
    print("4. Smaller models like granite3.3:2b should easily fit in GPU")

if __name__ == '__main__':
    main()