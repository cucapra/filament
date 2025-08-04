#!/usr/bin/env python3
"""
Vivado Critical Path Visualizer

Creates an interactive HTML visualization of critical path timing data.
Usage: python visualize_timing.py <input.rpt> [output.html]
"""

import sys
import json
import subprocess
from pathlib import Path

def parse_rpt_file(rpt_file):
    """Parse RPT file using the existing parse.py script"""
    try:
        result = subprocess.run(['python', 'parse.py', rpt_file], 
                              capture_output=True, text=True, check=True)
        return json.loads(result.stdout)
    except subprocess.CalledProcessError as e:
        print(f"Error parsing RPT file: {e}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON output: {e}")
        sys.exit(1)

def generate_html(critical_paths, output_file):
    """Generate interactive HTML visualization"""
    
    # Extract all unique resource types for legend
    resource_types = set()
    for path in critical_paths:
        for segment in path['critical_path']:
            resource_types.add(segment['kind'])
    
    resource_types = sorted(list(resource_types))
    
    # Define colors for different resource types
    colors = [
        '#FF6B6B', '#4ECDC4', '#45B7D1', '#96CEB4', '#FFEAA7',
        '#DDA0DD', '#98D8C8', '#F7DC6F', '#BB8FCE', '#85C1E9',
        '#F8C471', '#82E0AA', '#F1948A', '#85C1E9', '#D2B4DE'
    ]
    
    color_map = {rt: colors[i % len(colors)] for i, rt in enumerate(resource_types)}
    
    html_content = f"""
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Vivado Critical Path Visualization</title>
    <style>
        body {{
            font-family: Arial, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }}
        
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        
        .controls {{
            margin-bottom: 20px;
            background: white;
            padding: 15px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        
        .legend {{
            display: flex;
            flex-wrap: wrap;
            gap: 10px;
            margin-top: 10px;
        }}
        
        .legend-item {{
            display: flex;
            align-items: center;
            cursor: pointer;
            padding: 5px 10px;
            border-radius: 5px;
            transition: opacity 0.3s;
        }}
        
        .legend-item.disabled {{
            opacity: 0.3;
        }}
        
        .legend-color {{
            width: 20px;
            height: 20px;
            margin-right: 8px;
            border-radius: 3px;
        }}
        
        .path-container {{
            background: white;
            margin-bottom: 15px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            overflow: hidden;
        }}
        
        .path-header {{
            background: #2c3e50;
            color: white;
            padding: 10px 15px;
            font-weight: bold;
        }}
        
        .path-info {{
            font-size: 0.9em;
            margin-top: 5px;
        }}
        
        .path-bar {{
            position: relative;
            height: 40px;
            margin: 15px;
            border: 1px solid #ddd;
            border-radius: 5px;
            overflow: hidden;
        }}
        
        .path-segment {{
            position: absolute;
            height: 100%;
            cursor: pointer;
            border-right: 1px solid rgba(255,255,255,0.3);
            transition: opacity 0.3s;
        }}
        
        .path-segment:hover {{
            opacity: 0.8;
            outline: 2px solid #333;
        }}
        
        .path-segment.hidden {{
            display: none;
        }}
        
        .tooltip {{
            position: absolute;
            background: rgba(0,0,0,0.9);
            color: white;
            padding: 10px;
            border-radius: 5px;
            font-size: 12px;
            pointer-events: none;
            z-index: 1000;
            max-width: 300px;
            display: none;
        }}
        
        .segment-details {{
            background: #f8f9fa;
            padding: 15px;
            margin: 0 15px 15px;
            border-radius: 5px;
            display: none;
        }}
        
        .segment-details.show {{
            display: block;
        }}

        .timing-summary {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 10px;
            margin-bottom: 10px;
        }}

        .timing-item {{
            background: #ecf0f1;
            padding: 8px;
            border-radius: 4px;
            text-align: center;
        }}

        .timing-label {{
            font-size: 0.8em;
            color: #666;
        }}

        .timing-value {{
            font-weight: bold;
            font-size: 1.1em;
        }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Vivado Critical Path Visualization</h1>
        <p>Interactive timing analysis of critical paths</p>
    </div>

    <div class="controls">
        <h3>Legend & Controls</h3>
        <p>Click on resource types to toggle visibility:</p>
        <div class="legend">
            {' '.join([f'<div class="legend-item" data-type="{rt}"><div class="legend-color" style="background-color: {color_map[rt]};"></div><span>{rt}</span></div>' for rt in resource_types])}
        </div>
    </div>

    <div id="paths-container">
"""

    # Generate path visualizations
    for i, path in enumerate(critical_paths):
        header = path['header']
        critical_path = path['critical_path']
        
        # Calculate total delay and create segments
        total_delay = header['data_path_delay']['total']
        
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
        
        # Create segments for each resource in the critical path
        cumulative_delay = 0
        for j, segment in enumerate(critical_path):
            segment_delay = segment['logic_delay'] + segment.get('net_delay', 0)
            width_percent = (segment_delay / total_delay) * 100
            left_percent = (cumulative_delay / total_delay) * 100
            
            segment_info = {
                'kind': segment['kind'],
                'resource_name': segment['resource_name'],
                'logic_delay': segment['logic_delay'],
                'net_delay': segment.get('net_delay', 0),
                'total_delay': segment_delay,
                'delay_type': segment.get('delay_type', ''),
                'location': f"({segment['loc']['x']}, {segment['loc']['y']})",
                'fan_out': segment.get('fan_out', 0),
                'net_name': segment.get('net_name', '')
            }
            
            html_content += f"""
                <div class="path-segment" 
                     data-type="{segment['kind']}"
                     data-info='{json.dumps(segment_info)}'
                     style="left: {left_percent:.2f}%; width: {width_percent:.2f}%; background-color: {color_map[segment['kind']]};"
                     onclick="toggleSegmentDetails('path-{i}', {j})">
                </div>
"""
            cumulative_delay += segment_delay
        
        html_content += f"""
            </div>
            <div class="segment-details" id="details-path-{i}">
                <h4>Segment Details</h4>
                <div id="details-content-path-{i}"></div>
            </div>
        </div>
"""

    html_content += """
    </div>

    <div class="tooltip" id="tooltip"></div>

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

        // Hover tooltip functionality
        document.addEventListener('mouseover', function(e) {
            if (e.target.classList.contains('path-segment')) {
                const info = JSON.parse(e.target.dataset.info);
                const tooltip = document.getElementById('tooltip');
                
                tooltip.innerHTML = `
                    <strong>${info.kind}</strong><br>
                    Resource: ${info.resource_name}<br>
                    Location: ${info.location}<br>
                    Logic Delay: ${info.logic_delay.toFixed(3)} ns<br>
                    Net Delay: ${info.net_delay.toFixed(3)} ns<br>
                    Total: ${info.total_delay.toFixed(3)} ns<br>
                    Fan-out: ${info.fan_out}
                `;
                
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
                
                contentDiv.innerHTML = `
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px;">
                        <div>
                            <h5>Resource Information</h5>
                            <p><strong>Type:</strong> ${info.kind}</p>
                            <p><strong>Name:</strong> ${info.resource_name}</p>
                            <p><strong>Location:</strong> ${info.location}</p>
                            <p><strong>Delay Type:</strong> ${info.delay_type}</p>
                        </div>
                        <div>
                            <h5>Timing Information</h5>
                            <p><strong>Logic Delay:</strong> ${info.logic_delay.toFixed(3)} ns</p>
                            <p><strong>Net Delay:</strong> ${info.net_delay.toFixed(3)} ns</p>
                            <p><strong>Total Delay:</strong> ${info.total_delay.toFixed(3)} ns</p>
                        </div>
                        <div>
                            <h5>Connectivity</h5>
                            <p><strong>Fan-out:</strong> ${info.fan_out}</p>
                            <p><strong>Net Name:</strong> ${info.net_name || 'N/A'}</p>
                        </div>
                    </div>
                `;
                
                detailsDiv.classList.add('show');
            }
        }

        // Initialize
        updatePathVisibility();
    </script>
</body>
</html>
"""

    with open(output_file, 'w') as f:
        f.write(html_content)

def main():
    if len(sys.argv) < 2:
        print("Usage: python visualize_timing.py <input.rpt> [output.html]")
        sys.exit(1)
    
    rpt_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else 'timing_visualization.html'
    
    if not Path(rpt_file).exists():
        print(f"Error: Input file '{rpt_file}' not found")
        sys.exit(1)
    
    print(f"Parsing RPT file: {rpt_file}")
    critical_paths = parse_rpt_file(rpt_file)
    
    print(f"Found {len(critical_paths)} critical paths")
    print(f"Generating HTML visualization: {output_file}")
    
    generate_html(critical_paths, output_file)
    print(f"Visualization saved to {output_file}")

if __name__ == "__main__":
    main()