#!/usr/bin/env python3
import sys
import os
from PIL import Image

def generate_ico(source_path, output_path):
    if not os.path.exists(source_path):
        print(f"Error: Source file {source_path} not found.")
        sys.exit(1)

    try:
        img = Image.open(source_path)
        # Create directory if it doesn't exist
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        
        # Save as ICO (PIL handles resizing automatically if needed, but explicit sizes are better for quality)
        # Standard Windows icon sizes
        sizes = [
            (16, 16),
            (32, 32),
            (48, 48),
            (64, 64),
            (128, 128),
            (256, 256)
        ]
        
        img.save(output_path, format='ICO', sizes=sizes)
        print(f"Successfully generated {output_path} from {source_path}")
        
    except Exception as e:
        print(f"Error converting icon: {e}")
        # If PIL is not installed, we might need a fallback or instruct user to install it.
        # But mac usually has python3, but PIL/pillow might not be there.
        # Fallback: Check if we can use a simpler method if PIL fails.
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: generate_ico.py <source_png> <output_ico>")
        sys.exit(1)
    
    generate_ico(sys.argv[1], sys.argv[2])
