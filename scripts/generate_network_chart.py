#!/usr/bin/env python3
"""
Generate a network speed bar chart image matching the style of black-box's network visualization.
Creates an 830px wide, transparent background image for use in README.
"""

from pathlib import Path
from PIL import Image, ImageDraw
import random

# Configuration
WIDTH = 830
HEIGHT = 10
NUM_BARS = 215  # Match original bar width: 60 bars @ 232px = 215 bars @ 830px
TRANSPARENT = (0, 0, 0, 0)
BACKGROUND = (249, 250, 251, 255)  # gray-50 from the web UI
REFERENCE_IMAGE = Path(__file__).resolve().parent.parent / "example.png"
VERTICAL_SUPERSAMPLE = 4
_REFERENCE_PROFILE = None
MANUAL_SEED_PROFILE = [
    0.50, 0.50, 0.57, 0.80, 0.50, 0.40, 0.70, 0.47, 0.47, 0.70,
    0.70, 0.70, 0.62, 0.60, 0.60, 0.47, 0.70, 0.60, 0.30, 0.50,
    0.42, 0.70, 0.53, 0.60, 0.65, 0.80, 0.70, 0.70, 0.55, 0.80,
    0.62, 0.70, 0.60, 0.40, 0.40, 0.80, 0.72, 0.80, 0.70, 0.40,
    0.35, 0.80, 1.00, 0.78, 0.40, 0.60, 0.68, 0.90, 0.50, 0.20,
    0.60, 0.60, 1.00, 0.40, 0.40, 0.35, 0.50, 0.80, 0.80, 0.65,
]

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


def lighten_color(color, amount=0.28):
    """Blend a color toward white for the bar's lighter edge."""
    return tuple(
        int(channel + ((255 - channel) * amount))
        for channel in color
    )


def extract_reference_profile(image_path):
    """
    Extract a normalized bar-height profile from the example screenshot.

    We sample the screenshot column-by-column so the extracted distribution
    reflects the visible rhythm of the real chart across its full width.
    """
    img = Image.open(image_path).convert("RGBA")
    width, height = img.size
    bg_rgb = BACKGROUND[:3]

    non_bg = [
        (x, y)
        for y in range(height)
        for x in range(width)
        if img.getpixel((x, y))[:3] != bg_rgb
    ]
    if not non_bg:
        raise ValueError(f"No chart pixels found in {image_path}")

    left = min(x for x, _ in non_bg)
    right = max(x for x, _ in non_bg)
    bottom = max(y for _, y in non_bg)
    profile = []

    for x in range(left, right + 1):
        top = height
        found = False
        for y in range(height):
            if img.getpixel((x, y))[:3] != bg_rgb:
                top = min(top, y)
                found = True

        if found:
            profile.append((bottom - top + 1) / HEIGHT)

    if not profile:
        raise ValueError(f"Unable to extract a reference profile from {image_path}")

    max_value = max(profile)
    return [value / max_value for value in profile]


def default_reference_profile():
    """
    Fallback profile measured from example.png.

    Manual 60-bar seed measured from example.png and normalized to the tallest
    visible bar.
    """
    return MANUAL_SEED_PROFILE[:]


def load_reference_profile():
    """Load the measured chart profile from example.png when available."""
    global _REFERENCE_PROFILE
    if _REFERENCE_PROFILE is not None:
        return _REFERENCE_PROFILE

    print(f"Using manual seed profile ({len(MANUAL_SEED_PROFILE)} bars)")
    _REFERENCE_PROFILE = default_reference_profile()
    return _REFERENCE_PROFILE

class NetworkActivityModel:
    """Continuous activity model guided by the measured screenshot profile."""

    def __init__(self, reference):
        self.reference = reference
        self.position = random.uniform(0, len(reference) - 1)
        self.speed = random.uniform(0.85, 1.2)
        self.current = self._reference_value(self.position)
        self.drift = 0.0
        self.micro = 0.0
        self.impulse = 0.0

    def _reference_value(self, position):
        left = int(position) % len(self.reference)
        right = (left + 1) % len(self.reference)
        frac = position - int(position)
        return (self.reference[left] * (1.0 - frac)) + (self.reference[right] * frac)

    def next_value(self):
        if random.random() < 0.035:
            self.position = random.uniform(0, len(self.reference) - 1)
            self.speed = random.uniform(0.8, 1.25)

        target = self._reference_value(self.position)
        self.position = (self.position + self.speed + random.uniform(-0.08, 0.08)) % len(self.reference)
        self.speed = min(1.3, max(0.7, self.speed + random.uniform(-0.03, 0.03)))

        # Low-frequency drift helps avoid repeated exact plateaus.
        self.drift = (self.drift * 0.82) + random.uniform(-0.035, 0.035)
        # High-frequency wobble breaks up equal-height neighbors once the
        # 60-bar seed is expanded to README width.
        self.micro = (self.micro * 0.18) + random.uniform(-0.11, 0.11)
        # Short-lived impulses create the single-bar jumps seen in the sample.
        self.impulse *= 0.10
        if random.random() < 0.15:
            direction = -1.0 if random.random() < 0.6 else 1.0
            strength = random.uniform(0.18, 0.38) if direction < 0 else random.uniform(0.14, 0.30)
            self.impulse += direction * strength

        value = (self.current * 0.32) + (target * 0.68)
        value += self.drift + self.micro + self.impulse + random.uniform(-0.05, 0.05)

        if random.random() < 0.14:
            value -= random.uniform(0.14, 0.34)
        if random.random() < 0.035:
            value += random.uniform(0.08, 0.18)

        # Keep a visible green tail while preserving the screenshot's mostly
        # warm middle. The UI rescales per frame, so these relative values are
        # what drives the apparent color distribution.
        value = min(1.0, max(0.06, value))
        self.current = value
        return 100.0 * value


def generate_varied_network_data(num_points, model=None):
    """Generate continuous chart samples shaped by the measured reference."""
    model = model or NetworkActivityModel(load_reference_profile())
    return [model.next_value() for _ in range(num_points)]


def render_chart_image(data):
    """
    Render the chart with extra vertical resolution, then downsample.

    This preserves continuous-looking height variation even though the final
    README asset is only 10px tall.
    """
    render_height = HEIGHT * VERTICAL_SUPERSAMPLE
    img = Image.new('RGBA', (WIDTH, render_height), BACKGROUND)
    draw = ImageDraw.Draw(img)

    bar_width = WIDTH / len(data)
    max_val = max(data) if data else 1

    prev_rendered = None
    for i, val in enumerate(data):
        normalized = val / max_val
        # Small render-time jitter avoids visible plateaus where adjacent bars
        # fall into the same color band but should not look identical.
        normalized = min(1.0, max(0.0, normalized + random.uniform(-0.035, 0.035)))
        if prev_rendered is not None and abs(normalized - prev_rendered) < 0.022:
            nudge = random.uniform(0.022, 0.055)
            normalized += nudge if random.random() < 0.5 else -nudge
            normalized = min(1.0, max(0.0, normalized))
        bar_height = normalized * render_height
        pct = normalized * 100
        color = get_usage_color(pct)
        edge_color = lighten_color(color)
        x = i * bar_width
        y = render_height - bar_height
        edge_width = max(1.0, bar_width * 0.22)
        body_end = max(x, (x + bar_width) - edge_width)
        draw.rectangle([(x, y), (body_end, render_height)], fill=color)
        draw.rectangle([(body_end, y), (x + bar_width, render_height)], fill=edge_color)
        prev_rendered = normalized

    return img.resize((WIDTH, HEIGHT), Image.Resampling.BICUBIC)

def draw_network_chart(filename, data):
    """
    Draw a network chart matching the style of black-box's visualization.

    Args:
        filename: Output PNG filename
        data: List of network speed values (0-100)
    """
    img = render_chart_image(data)
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
    return render_chart_image(data)

def generate_animated_chart(filename, num_frames=60):
    """
    Generate an animated GIF where bars shift left and new bars appear on the right.

    Args:
        filename: Output GIF filename
        num_frames: Number of frames to generate (default 60 for 60 seconds)
    """
    print(f"Generating animated chart with {num_frames} frames...")

    model = NetworkActivityModel(load_reference_profile())
    data = generate_varied_network_data(NUM_BARS, model=model)

    frames = []

    for frame_num in range(num_frames):
        # Create frame with current data
        frame = create_network_chart_frame(data)
        frames.append(frame)

        # Shift data left and add a new value from the same evolving model.
        data = data[1:]  # Remove leftmost bar
        data.append(model.next_value())

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
