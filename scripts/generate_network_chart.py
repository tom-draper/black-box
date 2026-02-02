#!/usr/bin/env python3
"""
Generate a network speed bar chart image matching the style of black-box's network visualization.
Creates an 830px wide, transparent background image for use in README.
"""

from PIL import Image, ImageDraw
import random
import math

# Configuration
WIDTH = 830
HEIGHT = 10
NUM_BARS = 215  # Match original bar width: 60 bars @ 232px = 215 bars @ 830px
TRANSPARENT = (0, 0, 0, 0)

# Tailwind color mapping (same as getUsageColor function)
COLOR_MAP = [
    (90, (239, 68, 68)),    # red-500
    (80, (248, 113, 113)),  # red-400
    (70, (252, 165, 165)),  # red-300
    (60, (234, 179, 8)),    # yellow-500
    (50, (250, 204, 21)),   # yellow-400
    (40, (253, 224, 71)),   # yellow-300
    (30, (163, 230, 53)),   # lime-400
    (20, (132, 204, 22)),   # lime-500
    (10, (34, 197, 94)),    # green-500
    (0, (74, 222, 128)),    # green-400
]

def get_usage_color(pct):
    """Get color based on usage percentage (0-100)."""
    for threshold, color in COLOR_MAP:
        if pct >= threshold:
            return color
    return COLOR_MAP[-1][1]  # default green-400

def generate_varied_network_data(num_points):
    """
    Generate simulated network speed data with natural-looking spikes.
    Creates a low baseline with occasional red and yellow spikes that ramp up/down.
    """
    data = []
    i = 0

    while i < num_points:
        # Normal baseline activity
        baseline_length = random.randint(5, 15)

        for _ in range(min(baseline_length, num_points - i)):
            # Low baseline with some variation (10-30%)
            value = random.uniform(10, 30)
            # Add noise
            value += random.uniform(-8, 8)
            value = max(5, value)  # Keep above 5
            data.append(value)
            i += 1
            if i >= num_points:
                break

        # 40% chance of a spike cluster
        if random.random() > 0.6 and i < num_points:
            spike_type = random.random()

            if spike_type > 0.7:
                # Red spike cluster (high activity)
                peak = random.uniform(75, 100)
                spike_length = random.randint(2, 5)
            else:
                # Yellow spike cluster (medium activity)
                peak = random.uniform(45, 70)
                spike_length = random.randint(3, 7)

            # Ramp up
            ramp_up = random.randint(2, 4)
            for j in range(min(ramp_up, num_points - i)):
                value = random.uniform(20, peak * (j + 1) / ramp_up)
                # Add noise
                value += random.uniform(-10, 10)
                value = max(15, min(peak, value))
                data.append(value)
                i += 1
                if i >= num_points:
                    break

            # Peak
            for _ in range(min(spike_length, num_points - i)):
                value = random.uniform(peak * 0.8, peak)
                # Add noise
                value += random.uniform(-8, 8)
                value = max(peak * 0.7, min(100, value))
                data.append(value)
                i += 1
                if i >= num_points:
                    break

            # Ramp down
            ramp_down = random.randint(2, 4)
            for j in range(min(ramp_down, num_points - i)):
                value = random.uniform(peak * (ramp_down - j) / ramp_down, 30)
                # Add noise
                value += random.uniform(-10, 10)
                value = max(10, value)
                data.append(value)
                i += 1
                if i >= num_points:
                    break

    return data[:num_points]

def draw_network_chart(filename, data):
    """
    Draw a network chart matching the style of black-box's visualization.

    Args:
        filename: Output PNG filename
        data: List of network speed values (0-100)
    """
    # Create image with RGBA for transparency
    img = Image.new('RGBA', (WIDTH, HEIGHT), TRANSPARENT)
    draw = ImageDraw.Draw(img)

    # Calculate bar width
    bar_width = WIDTH / len(data)

    # Find max value for scaling
    max_val = max(data) if data else 1

    # Draw each bar
    for i, val in enumerate(data):
        # Calculate bar height (scaled to canvas height)
        bar_height = (val / max_val) * HEIGHT

        # Calculate percentage for color (relative to max)
        pct = (val / max_val) * 100

        # Get color
        color = get_usage_color(pct)

        # Calculate bar position
        x = i * bar_width
        y = HEIGHT - bar_height  # Draw from bottom

        # Draw bar (PIL uses x0, y0, x1, y1)
        draw.rectangle(
            [(x, y), (x + bar_width, HEIGHT)],
            fill=color
        )

    # Save image
    img.save(filename, 'PNG')
    print(f"Generated {filename} ({WIDTH}x{HEIGHT}px, {len(data)} bars)")

def create_network_chart_frame(data):
    """
    Create a single frame of the network chart (returns PIL Image).

    Args:
        data: List of network speed values (0-100)

    Returns:
        PIL Image object
    """
    # Create image with RGBA for transparency
    img = Image.new('RGBA', (WIDTH, HEIGHT), TRANSPARENT)
    draw = ImageDraw.Draw(img)

    # Calculate bar width
    bar_width = WIDTH / len(data)

    # Find max value for scaling
    max_val = max(data) if data else 1

    # Draw each bar
    for i, val in enumerate(data):
        # Calculate bar height (scaled to canvas height)
        bar_height = (val / max_val) * HEIGHT

        # Calculate percentage for color (relative to max)
        pct = (val / max_val) * 100

        # Get color
        color = get_usage_color(pct)

        # Calculate bar position
        x = i * bar_width
        y = HEIGHT - bar_height  # Draw from bottom

        # Draw bar (PIL uses x0, y0, x1, y1)
        draw.rectangle(
            [(x, y), (x + bar_width, HEIGHT)],
            fill=color
        )

    return img

def generate_animated_chart(filename, num_frames=60):
    """
    Generate an animated GIF where bars shift left and new bars appear on the right.

    Args:
        filename: Output GIF filename
        num_frames: Number of frames to generate (default 60 for 60 seconds)
    """
    print(f"Generating animated chart with {num_frames} frames...")

    # Start with initial data
    data = generate_varied_network_data(NUM_BARS)

    frames = []

    for frame_num in range(num_frames):
        # Create frame with current data
        frame = create_network_chart_frame(data)
        frames.append(frame)

        # Shift data left and add new bar on right
        data = data[1:]  # Remove leftmost bar

        # Generate new bar value using the same pattern generation
        # We'll generate a small chunk and take the last value to maintain pattern
        new_chunk = generate_varied_network_data(5)
        data.append(new_chunk[-1])

        if (frame_num + 1) % 10 == 0:
            print(f"Generated {frame_num + 1}/{num_frames} frames...")

    # Save as animated GIF (1000ms = 1 second per frame)
    frames[0].save(
        filename,
        save_all=True,
        append_images=frames[1:],
        duration=1000,  # 1 second per frame
        loop=0,  # Loop forever
        transparency=0,
        disposal=2
    )

    print(f"Generated {filename} ({WIDTH}x{HEIGHT}px, {num_frames} frames, 1 sec/frame)")

def main():
    """Generate network chart images for README."""

    # Generate simulated data
    print("Generating simulated network data...")
    network_data = generate_varied_network_data(NUM_BARS)

    # Show statistics to verify randomness
    print(f"Data stats: min={min(network_data):.1f}, max={max(network_data):.1f}, avg={sum(network_data)/len(network_data):.1f}")
    print(f"Sample values: {[round(v, 1) for v in network_data[:10]]}")

    # Create chart
    print("Drawing network chart...")
    draw_network_chart('network_chart.png', network_data)

    print("\nChart generated successfully!")
    print("You can now use 'network_chart.png' in your README.")

    # Generate a second variation for download/upload pair
    print("\nGenerating second chart for comparison...")
    network_data2 = generate_varied_network_data(NUM_BARS)
    print(f"Data stats: min={min(network_data2):.1f}, max={max(network_data2):.1f}, avg={sum(network_data2)/len(network_data2):.1f}")
    print(f"Sample values: {[round(v, 1) for v in network_data2[:10]]}")
    draw_network_chart('network_chart_alt.png', network_data2)

    print("\nBoth charts generated!")
    print("- network_chart.png (first variation)")
    print("- network_chart_alt.png (second variation)")

    # Generate animated version (same number of frames as bars)
    print("\nGenerating animated GIF...")
    generate_animated_chart('network_chart_animated.gif', num_frames=NUM_BARS)

    print("\nAll charts generated!")
    print("- network_chart.png (static)")
    print("- network_chart_alt.png (static)")
    print(f"- network_chart_animated.gif ({NUM_BARS}-second animation)")

if __name__ == '__main__':
    main()
