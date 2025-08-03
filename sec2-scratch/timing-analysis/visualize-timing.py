#!/usr/bin/env python3
"""
Vivado Critical Path Visualizer - Fixed Version

Creates an interactive HTML visualization of critical path timing data.
Usage: python visualize_timing_fixed.py <input.rpt> [output.html]
"""

import sys
import json
import subprocess
from pathlib import Path

css = f"""
    <style>
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f8f9fa;
            color: #2c3e50;
        }}

        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}

        .header h1 {{
            color: #2c3e50;
            margin-bottom: 10px;
        }}

        .controls {{
            margin-bottom: 20px;
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}

        .legend {{
            display: flex;
            flex-wrap: wrap;
            gap: 12px;
            margin-top: 15px;
        }}

        .legend-item {{
            display: flex;
            align-items: center;
            cursor: pointer;
            padding: 8px 12px;
            border-radius: 6px;
            border: 2px solid transparent;
            transition: all 0.2s ease;
            background: #f8f9fa;
        }}

        .legend-item:hover {{
            border-color: #3498db;
            background-color: #ecf0f1;
        }}

        .legend-item.disabled {{
            opacity: 0.3;
            background-color: #e9ecef;
        }}

        .legend-color {{
            width: 24px;
            height: 24px;
            margin-right: 10px;
            border-radius: 4px;
            border: 1px solid #bdc3c7;
        }}

        .path-container {{
            background: white;
            margin-bottom: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            overflow: hidden;
        }}

        .path-header {{
            background: #34495e;
            color: white;
            padding: 15px 20px;
            font-weight: 600;
        }}

        .path-info {{
            font-size: 0.9em;
            margin-top: 10px;
        }}

        .path-bar {{
            position: relative;
            height: 50px;
            margin: 20px 20px 10px 20px;
            border: 2px solid #bdc3c7;
            border-radius: 6px;
            overflow: hidden;
            background: #ecf0f1;
        }}

        .x-axis {{
            position: relative;
            height: 30px;
            margin: 0 20px 20px 20px;
            font-size: 10px;
            color: #7f8c8d;
        }}

        .x-axis-tick {{
            position: absolute;
            height: 100%;
            border-left: 1px solid #bdc3c7;
            display: flex;
            align-items: flex-end;
            padding-bottom: 2px;
        }}

        .x-axis-label {{
            font-size: 9px;
            transform: translateX(-50%);
            white-space: nowrap;
        }}

        .path-segment {{
            position: absolute;
            height: 100%;
            cursor: pointer;
            border-right: 2px solid #2c3e50;
            transition: all 0.2s ease;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 10px;
            font-weight: bold;
            color: #2c3e50;
            text-shadow: 0 0 3px rgba(255,255,255,0.9);
            min-width: 3px;
            overflow: hidden;
        }}

        .path-segment:first-child {{
            border-left: none;
        }}

        .path-segment:last-child {{
            border-right: none;
        }}

        .path-segment:hover {{
            transform: translateY(-2px);
            box-shadow: 0 4px 8px rgba(0,0,0,0.2);
            z-index: 10;
            border-color: #e74c3c;
        }}

        .path-segment.hidden {{
            display: none;
        }}

        .routing-segment {{
            background-image: repeating-linear-gradient(
                45deg,
                transparent,
                transparent 3px,
                rgba(255,255,255,0.3) 3px,
                rgba(255,255,255,0.3) 6px
            );
            font-style: italic;
        }}

        .tooltip {{
            position: absolute;
            background: rgba(0,0,0,0.95);
            color: white;
            padding: 12px;
            border-radius: 6px;
            font-size: 12px;
            pointer-events: none;
            z-index: 1000;
            max-width: 320px;
            display: none;
            border: 1px solid #34495e;
        }}

        .segment-details {{
            background: #f8f9fa;
            padding: 20px;
            margin: 0 20px 20px;
            border-radius: 6px;
            display: none;
            border: 1px solid #e9ecef;
        }}

        .segment-details.show {{
            display: block;
        }}

        .timing-summary {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
            gap: 12px;
            margin-bottom: 15px;
        }}

        .timing-item {{
            background: #ecf0f1;
            padding: 12px;
            border-radius: 6px;
            text-align: center;
            border: 1px solid #d5dbdb;
        }}

        .timing-label {{
            font-size: 0.8em;
            color: #7f8c8d;
            font-weight: 500;
            margin-bottom: 4px;
        }}

        .timing-value {{
            font-weight: bold;
            font-size: 1.1em;
            color: #2c3e50;
        }}

        .segment-text {{
            font-size: 9px;
            text-overflow: ellipsis;
            white-space: nowrap;
            overflow: hidden;
        }}

        /* Mobile responsive */
        @media (max-width: 768px) {{
            .legend {{
                flex-direction: column;
                gap: 8px;
            }}

            .legend-item {{
                justify-content: flex-start;
                padding: 12px;
            }}

            .legend-color {{
                width: 30px;
                height: 30px;
            }}

            .timing-summary {{
                grid-template-columns: 1fr 1fr;
                gap: 8px;
            }}

            .path-bar {{
                height: 60px;
                margin: 15px 10px 10px 10px;
            }}

            .x-axis {{
                margin: 0 10px 15px 10px;
            }}
        }}
    </style>
"""
javascript = """
    <script>
        let disabledTypes = new Set();
        let selectedSegments = new Map(); // Track selected segments per path

        // Legend toggle functionality
        document.querySelectorAll('.legend-item').forEach(item => {
            item.addEventListener('click', function() {
                const type = this.dataset.type;

                if (disabledTypes.has(type)) {
                    disabledTypes.delete(type);
                    this.classList.remove('disabled');
                } else {
                    disabledTypes.add(type);
                    this.classList.add('disabled');
                }

                updatePathVisibility();
            });
        });

        function updatePathVisibility() {
            document.querySelectorAll('.path-segment').forEach(segment => {
                const type = segment.dataset.type;
                if (disabledTypes.has(type)) {
                    segment.classList.add('hidden');
                } else {
                    segment.classList.remove('hidden');
                }
            });

            // Recalculate bar scales for visible segments
            document.querySelectorAll('.path-bar').forEach(recalculatePathScale);

            // Update x-axis for each path
            document.querySelectorAll('.path-bar').forEach((pathBar, index) => {
                updateXAxis(pathBar, index);
            });
        }

        function recalculatePathScale(pathBar) {
            const segments = Array.from(pathBar.querySelectorAll('.path-segment:not(.hidden)'));
            if (segments.length === 0) return;

            // Calculate total delay of visible segments
            let totalVisibleDelay = 0;
            segments.forEach(segment => {
                const info = JSON.parse(segment.dataset.info);
                totalVisibleDelay += info.total_delay;
            });

            // Redistribute segments
            let cumulativeDelay = 0;
            segments.forEach(segment => {
                const info = JSON.parse(segment.dataset.info);
                const leftPercent = (cumulativeDelay / totalVisibleDelay) * 100;
                const widthPercent = (info.total_delay / totalVisibleDelay) * 100;

                segment.style.left = leftPercent + '%';
                segment.style.width = widthPercent + '%';

                cumulativeDelay += info.total_delay;
            });
        }

        // Enhanced hover tooltip functionality
        document.addEventListener('mouseover', function(e) {
            if (e.target.classList.contains('path-segment')) {
                const info = JSON.parse(e.target.dataset.info);
                const tooltip = document.getElementById('tooltip');

                let tooltipContent = `<strong>${info.kind}</strong><br>`;
                tooltipContent += `Resource: ${info.resource_name}<br>`;
                tooltipContent += `Location: ${info.location}<br>`;

                if (info.kind === 'ROUTING') {
                    tooltipContent += `Net Delay: ${info.net_delay.toFixed(3)} ns<br>`;
                    tooltipContent += `Total: ${info.total_delay.toFixed(3)} ns<br>`;
                    tooltipContent += `Fan-out: ${info.fan_out}`;
                } else {
                    tooltipContent += `Logic Delay: ${info.logic_delay.toFixed(3)} ns<br>`;
                    if (info.net_delay > 0) {
                        tooltipContent += `Net Delay: ${info.net_delay.toFixed(3)} ns<br>`;
                    }
                    tooltipContent += `Total: ${info.total_delay.toFixed(3)} ns<br>`;
                    tooltipContent += `Fan-out: ${info.fan_out}`;
                }

                tooltip.innerHTML = tooltipContent;
                tooltip.style.display = 'block';
                tooltip.style.left = e.pageX + 10 + 'px';
                tooltip.style.top = e.pageY + 10 + 'px';
            }
        });

        document.addEventListener('mouseout', function(e) {
            if (e.target.classList.contains('path-segment')) {
                document.getElementById('tooltip').style.display = 'none';
            }
        });

        document.addEventListener('mousemove', function(e) {
            const tooltip = document.getElementById('tooltip');
            if (tooltip.style.display === 'block') {
                tooltip.style.left = e.pageX + 10 + 'px';
                tooltip.style.top = e.pageY + 10 + 'px';
            }
        });

        // Click to show/hide detailed information
        function toggleSegmentDetails(pathId, segmentIndex) {
            const detailsDiv = document.getElementById('details-' + pathId);
            const contentDiv = document.getElementById('details-content-' + pathId);
            const pathBar = document.getElementById(pathId);
            const segments = pathBar.querySelectorAll('.path-segment');
            const segment = segments[segmentIndex];
            const info = JSON.parse(segment.dataset.info);

            // Check if this segment is already selected
            const isCurrentlySelected = selectedSegments.get(pathId) === segmentIndex;

            if (isCurrentlySelected) {
                // Deselect
                detailsDiv.classList.remove('show');
                selectedSegments.delete(pathId);
            } else {
                // Select new segment
                selectedSegments.set(pathId, segmentIndex);

                let detailsContent = `
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px;">
                        <div>
                            <h5>Resource Information</h5>
                            <p><strong>Type:</strong> ${info.kind}</p>
                            <p><strong>Name:</strong> ${info.resource_name}</p>
                            <p><strong>Location:</strong> ${info.location}</p>
                            <p><strong>Delay Type:</strong> ${info.delay_type}</p>
                        </div>
                        <div>
                            <h5>Timing Information</h5>`;

                if (info.kind === 'ROUTING') {
                    detailsContent += `
                            <p><strong>Net Delay:</strong> ${info.net_delay.toFixed(3)} ns</p>
                            <p><strong>Total Delay:</strong> ${info.total_delay.toFixed(3)} ns</p>`;
                } else {
                    detailsContent += `
                            <p><strong>Logic Delay:</strong> ${info.logic_delay.toFixed(3)} ns</p>
                            <p><strong>Net Delay:</strong> ${info.net_delay.toFixed(3)} ns</p>
                            <p><strong>Total Delay:</strong> ${info.total_delay.toFixed(3)} ns</p>`;
                }

                detailsContent += `
                        </div>
                        <div>
                            <h5>Connectivity</h5>
                            <p><strong>Fan-out:</strong> ${info.fan_out}</p>
                            <p><strong>Net Name:</strong> ${info.net_name || 'N/A'}</p>
                        </div>
                    </div>
                `;

                contentDiv.innerHTML = detailsContent;
                detailsDiv.classList.add('show');
            }
        }

        // X-axis generation and update function
        function updateXAxis(pathBar, pathIndex) {
            const segments = Array.from(pathBar.querySelectorAll('.path-segment:not(.hidden)'));
            if (segments.length === 0) return;

            // Calculate total delay of visible segments
            let totalVisibleDelay = 0;
            segments.forEach(segment => {
                const info = JSON.parse(segment.dataset.info);
                totalVisibleDelay += info.total_delay;
            });

            const xAxisDiv = document.getElementById(`x-axis-path-${pathIndex}`);
            xAxisDiv.innerHTML = ''; // Clear existing ticks

            // Generate ticks at reasonable intervals
            const tickCount = 5; // Number of ticks
            for (let i = 0; i <= tickCount; i++) {
                const tickValue = (totalVisibleDelay * i) / tickCount;
                const tickPosition = (i / tickCount) * 100; // Percentage position

                const tickDiv = document.createElement('div');
                tickDiv.className = 'x-axis-tick';
                tickDiv.style.left = tickPosition + '%';

                const labelDiv = document.createElement('div');
                labelDiv.className = 'x-axis-label';
                labelDiv.textContent = tickValue.toFixed(3) + ' ns';

                tickDiv.appendChild(labelDiv);
                xAxisDiv.appendChild(tickDiv);
            }
        }

        // Initialize
        updatePathVisibility();
    </script>
"""


def parse_rpt_file(rpt_file):
    """Parse RPT file using the existing parse.py script"""
    try:
        result = subprocess.run(
            ["python", "parse.py", rpt_file], capture_output=True, text=True, check=True
        )
        return json.loads(result.stdout)
    except subprocess.CalledProcessError as e:
        print(f"Error parsing RPT file: {e}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON output: {e}")
        sys.exit(1)


def generate_html(rpt_path, critical_paths, output_file):
    """Generate interactive HTML visualization"""

    # Extract all unique resource types for legend (including routing)
    resource_types = set()
    for path in critical_paths:
        for segment in path["critical_path"]:
            resource_types.add(segment["kind"])
            # Add routing as a resource type if there's net delay
            if segment.get("net_delay", 0) > 0:
                resource_types.add("ROUTING")

    resource_types = sorted(list(resource_types))

    # Define colors for different resource types with routing
    # LUT resources use same color family with different shades
    color_map = {
        "CARRY8": "#e74c3c",  # Red
        "FDRE": "#16a085",  # Teal
        "LUT2": "#4CAF50",  # Light green (LUT family)
        "LUT3": "#388E3C",  # Medium green (LUT family)
        "LUT4": "#2E7D32",  # Dark green (LUT family)
        "LUT5": "#1B5E20",  # Darker green (LUT family)
        "LUT6": "#0D4710",  # Darkest green (LUT family)
        "ROUTING": "#8B4513",  # Brown for routing
    }

    # Assign colors to any additional resource types
    additional_colors = ["#e67e22", "#34495e", "#95a5a6", "#d35400", "#7f8c8d"]
    color_idx = 0
    for rt in resource_types:
        if rt not in color_map:
            color_map[rt] = additional_colors[color_idx % len(additional_colors)]
            color_idx += 1

    html_content = f"""
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Vivado Critical Path Visualization</title>
    {css}
</head>
<body>
    <div class="header">
        <h1>Critical Path Visualization</h1>
        <p>{rpt_path}</p>
    </div>

    <div class="controls">
        <h3>Legend & Controls</h3>
        <p>Click on resource types to toggle visibility. Segments appear in critical path order:</p>
        <div class="legend">
            {' '.join([f'<div class="legend-item" data-type="{rt}"><div class="legend-color" style="background-color: {color_map[rt]};"></div><span>{rt}</span></div>' for rt in resource_types])}
        </div>
    </div>

    <div id="paths-container">
"""

    # Generate path visualizations with proper sequencing

    for i, path in enumerate(critical_paths):
        header = path["header"]
        critical_path = path["critical_path"]

        # Calculate total delay including routing
        total_delay = header["data_path_delay"]["total"]

        html_content += f"""
        <div class="path-container">
            <div class="path-header">
                Path {i + 1}: {header['source']} → {header['destination']}
                <div class="path-info">
                    <div class="timing-summary">
                        <div class="timing-item">
                            <div class="timing-label">Status</div>
                            <div class="timing-value">{header['status']}</div>
                        </div>
                        <div class="timing-item">
                            <div class="timing-label">Slack</div>
                            <div class="timing-value">{header['slack']:.3f} ns</div>
                        </div>
                        <div class="timing-item">
                            <div class="timing-label">Total Delay</div>
                            <div class="timing-value">{total_delay:.3f} ns</div>
                        </div>
                        <div class="timing-item">
                            <div class="timing-label">Logic</div>
                            <div class="timing-value">{header['data_path_delay']['logic']:.3f} ns ({header['data_path_delay']['logic_percent']:.1f}%)</div>
                        </div>
                        <div class="timing-item">
                            <div class="timing-label">Route</div>
                            <div class="timing-value">{header['data_path_delay']['route']:.3f} ns ({header['data_path_delay']['route_percent']:.1f}%)</div>
                        </div>
                        <div class="timing-item">
                            <div class="timing-label">Logic Levels</div>
                            <div class="timing-value">{header['logic_levels']['levels']}</div>
                        </div>
                    </div>
                </div>
            </div>
            <div class="path-bar" id="path-{i}">
"""

        # Create segments in the EXACT ORDER they appear in the critical path
        cumulative_delay = 0
        segment_index = 0

        for j, segment in enumerate(critical_path):
            logic_delay = segment["logic_delay"]
            net_delay = segment.get("net_delay", 0)

            # First, add the logic delay segment if it exists
            if logic_delay > 0:
                width_percent = (logic_delay / total_delay) * 100
                left_percent = (cumulative_delay / total_delay) * 100

                logic_info = {
                    "kind": segment["kind"],
                    "resource_name": segment["resource_name"],
                    "logic_delay": logic_delay,
                    "net_delay": 0,
                    "total_delay": logic_delay,
                    "delay_type": segment.get("delay_type", ""),
                    "location": f"({segment['loc']['x']}, {segment['loc']['y']})",
                    "fan_out": segment.get("fan_out", 0),
                    "net_name": segment.get("net_name", ""),
                    "segment_type": "logic",
                }

                # Remove resource labels as requested
                segment_label = ""

                html_content += f"""
                <div class="path-segment"
                     data-type="{segment['kind']}"
                     data-info='{json.dumps(logic_info)}'
                     style="left: {left_percent:.2f}%; width: {width_percent:.2f}%; background-color: {color_map[segment['kind']]};"
                     onclick="toggleSegmentDetails('path-{i}', {segment_index})">
                     {segment_label}
                </div>
"""
                cumulative_delay += logic_delay
                segment_index += 1

            # Then, add the routing delay segment if it exists
            if net_delay > 0:
                width_percent = (net_delay / total_delay) * 100
                left_percent = (cumulative_delay / total_delay) * 100

                routing_info = {
                    "kind": "ROUTING",
                    "resource_name": f"Net: {segment.get('net_name', 'Unknown')}",
                    "logic_delay": 0,
                    "net_delay": net_delay,
                    "total_delay": net_delay,
                    "delay_type": "Routing delay",
                    "location": f"From ({segment['loc']['x']}, {segment['loc']['y']})",
                    "fan_out": segment.get("fan_out", 0),
                    "net_name": segment.get("net_name", ""),
                    "segment_type": "routing",
                }

                # Remove resource labels as requested
                route_label = ""

                html_content += f"""
                <div class="path-segment routing-segment"
                     data-type="ROUTING"
                     data-info='{json.dumps(routing_info)}'
                     style="left: {left_percent:.2f}%; width: {width_percent:.2f}%; background-color: {color_map['ROUTING']};"
                     onclick="toggleSegmentDetails('path-{i}', {segment_index})">
                     {route_label}
                </div>
"""
                cumulative_delay += net_delay
                segment_index += 1

        html_content += f"""
            </div>
            <div class="x-axis" id="x-axis-path-{i}"></div>
            <div class="segment-details" id="details-path-{i}">
                <h4>Segment Details</h4>
                <div id="details-content-path-{i}"></div>
            </div>
        </div>
"""

    html_content += f"""
    </div>
    <div class="tooltip" id="tooltip"></div>
    {javascript}
    </body>
    </html>"""

    with open(output_file, "w") as f:
        f.write(html_content)


def main():
    if len(sys.argv) < 2:
        print("Usage: python visualize_timing_fixed.py <input.rpt> [output.html]")
        sys.exit(1)

    rpt_file = sys.argv[1]
    output_file = (
        sys.argv[2] if len(sys.argv) > 2 else "timing_visualization_fixed.html"
    )

    if not Path(rpt_file).exists():
        print(f"Error: Input file '{rpt_file}' not found")
        sys.exit(1)

    print(f"Parsing RPT file: {rpt_file}")
    critical_paths = parse_rpt_file(rpt_file)

    print(f"Found {len(critical_paths)} critical paths")

    generate_html(rpt_file, critical_paths, output_file)


if __name__ == "__main__":
    main()
