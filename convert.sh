#!/bin/sh

if [ $# -ne 2 ]; then
    echo "Usage: $0 <input.gif> <output_directory>"
    exit 1
fi

input_file="$1"
output_dir="$2"

if [ ! -f "$input_file" ]; then
    echo "Error: File '$input_file' does not exist."
    exit 1
fi

if [[ "$input_file" != *.gif ]]; then
    echo "Error: Input file must have a .gif extension."
    exit 1
fi

if [ ! -d "$output_dir" ]; then
    echo "Output directory '$output_dir' does not exist. Creating it..."
    mkdir -p "$output_dir"
    if [ $? -ne 0 ]; then
        echo "Error: Failed to create directory '$output_dir'."
        exit 1
    fi
fi

base_name=$(basename "$input_file" .gif)
output_file="$output_dir/${base_name}-bmp.bmp"

echo "Converting '$input_file' to '$output_file'..."
magick "$input_file" -coalesce "$output_file"

if [ $? -eq 0 ]; then
    echo "Success: File converted to '$output_file'."
else
    echo "Error: Conversion failed."
    exit 1
fi

