#!/usr/bin/env python3
"""
Vivado Critical Path Visualizer - Chart.js Clean Implementation

Creates an interactive HTML visualization of critical path timing data using Chart.js.
KEY CONSTRAINT: Resources appear in the EXACT order they occur in the critical path.

Usage: python visualize_timing_chartjs_clean.py <input.rpt> [output.html]
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

def get_color_map():
    """Get color mapping for resource types with LUT family gradients"""
    return {
        'CARRY8': '#e74c3c',     # Red
        'FDRE': '#16a085',       # Teal
        'LUT2': '#4CAF50',       # Light green (LUT family)
        'LUT3': '#388E3C',       # Medium green (LUT family)
        'LUT4': '#2E7D32',       # Dark green (LUT family)
        'LUT5': '#1B5E20',       # Darker green (LUT family)
        'LUT6': '#0D4710',       # Darkest green (LUT family)
        'ROUTING': '#8B4513',    # Brown for routing
    }

def extract_path_segments(critical_path):
    """Extract segments from critical path in sequential order"""
    segments = []
    
    for segment in critical_path:
        logic_delay = segment['logic_delay']
        net_delay = segment.get('net_delay', 0)
        
        # Add logic delay segment if it exists
        if logic_delay > 0:
            segments.append({
                'kind': segment['kind'],
                'resource_name': segment['resource_name'],
                'delay': logic_delay,
                'delay_type': segment.get('delay_type', ''),
                'location': f"({segment['loc']['x']}, {segment['loc']['y']})",
                'fan_out': segment.get('fan_out', 0),
                'net_name': segment.get('net_name', ''),
                'segment_type': 'logic'
            })
        
        # Add routing delay segment if it exists
        if net_delay > 0:
            segments.append({
                'kind': 'ROUTING',
                'resource_name': f"Net: {segment.get('net_name', 'Unknown')}",
                'delay': net_delay,
                'delay_type': 'Routing delay',
                'location': f"From ({segment['loc']['x']}, {segment['loc']['y']})",
                'fan_out': segment.get('fan_out', 0),
                'net_name': segment.get('net_name', ''),
                'segment_type': 'routing'
            })
    
    return segments

def extract_target_timing(critical_paths):
    """Extract target timing constraint from critical path data"""
    # Default target timing - can be extracted from clock constraints
    default_target = 7.0  # ns, based on typical FPGA clock period
    
    # Try to extract from first path's timing requirements
    # In real implementations, this could come from:
    # - Clock period constraints
    # - Timing requirements in the header
    # - User-specified targets
    if critical_paths and len(critical_paths) > 0:
        header = critical_paths[0]['header']
        # If there's a requirement field, use it, otherwise use default
        target = header.get('requirement', default_target)
        if target is None or target <= 0:
            target = default_target
    else:
        target = default_target
    
    return target

def generate_html(critical_paths, output_file):
    """Generate interactive HTML visualization using Chart.js"""
    
    color_map = get_color_map()
    
    # Extract target timing for red zone visualization
    target_timing = extract_target_timing(critical_paths)
    
    # Get all unique resource types for legend
    resource_types = set()
    for path in critical_paths:
        segments = extract_path_segments(path['critical_path'])
        for segment in segments:
            resource_types.add(segment['kind'])
    resource_types = sorted(list(resource_types))
    
    # Process paths data
    paths_data = []
    for i, path in enumerate(critical_paths):
        segments = extract_path_segments(path['critical_path'])
        paths_data.append({
            'path_index': i,
            'header': path['header'],
            'segments': segments
        })
    
    # Create the HTML template
    html_template = '''<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Vivado Critical Path Visualization - Chart.js Sequential</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.0/dist/chart.umd.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/chartjs-plugin-annotation@3.0.1/dist/chartjs-plugin-annotation.min.js"></script>
    <style>
        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f8f9fa;
            color: #2c3e50;
        }
        
        .header {
            text-align: center;
            margin-bottom: 30px;
        }
        
        .header h1 {
            color: #2c3e50;
            margin-bottom: 10px;
        }
        
        .controls {
            margin-bottom: 20px;
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        
        .legend {
            display: flex;
            flex-wrap: wrap;
            gap: 12px;
            margin-top: 15px;
        }
        
        .legend-item {
            display: flex;
            align-items: center;
            cursor: pointer;
            padding: 8px 12px;
            border-radius: 6px;
            border: 2px solid transparent;
            transition: all 0.2s ease;
            background: #f8f9fa;
        }
        
        .legend-item:hover {
            border-color: #3498db;
            background-color: #ecf0f1;
        }
        
        .legend-item.disabled {
            opacity: 0.3;
            background-color: #e9ecef;
        }
        
        .legend-color {
            width: 24px;
            height: 24px;
            margin-right: 10px;
            border-radius: 4px;
            border: 1px solid #bdc3c7;
        }
        
        .path-container {
            background: white;
            margin-bottom: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            overflow: hidden;
        }
        
        .path-header {
            background: #34495e;
            color: white;
            padding: 15px 20px;
            font-weight: 600;
        }
        
        .path-info {
            font-size: 0.9em;
            margin-top: 10px;
        }
        
        .timing-summary {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
            gap: 12px;
            margin-bottom: 15px;
        }

        .timing-item {
            background: #ecf0f1;
            padding: 12px;
            border-radius: 6px;
            text-align: center;
            border: 1px solid #d5dbdb;
        }

        .timing-label {
            font-size: 0.8em;
            color: #7f8c8d;
            font-weight: 500;
            margin-bottom: 4px;
        }

        .timing-value {
            font-weight: bold;
            font-size: 1.1em;
            color: #2c3e50;
        }
        
        .timing-value.violated {
            color: #c0392b;
            background-color: rgba(231, 76, 60, 0.1);
            padding: 4px 8px;
            border-radius: 4px;
        }
        
        .chart-container {
            position: relative;
            height: 120px;
            margin: 20px;
        }
        
        .segment-details {
            background: #f8f9fa;
            padding: 20px;
            margin: 0 20px 20px;
            border-radius: 6px;
            display: none;
            border: 1px solid #e9ecef;
        }
        
        .segment-details.show {
            display: block;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>Vivado Critical Path Visualization</h1>
        <p>Interactive timing analysis using Chart.js - Sequential critical path order</p>
    </div>

    <div class="controls">
        <h3>Legend & Controls</h3>
        <p>Click on resource types to toggle visibility. Segments appear in critical path order:</p>
        <div class="legend">
''' + ''.join([f'<div class="legend-item" data-type="{rt}"><div class="legend-color" style="background-color: {color_map.get(rt, "#666")};"></div><span>{rt}</span></div>' for rt in resource_types]) + '''
        </div>
        
    </div>

    <div id="paths-container">
'''

    # Add path containers
    for i, path in enumerate(critical_paths):
        header = path['header']
        html_template += f'''
        <div class="path-container">
            <div class="path-header">
                Path {i + 1}: {header['source']} → {header['destination']}
                <div class="path-info">
                    <div class="timing-summary">
                        <div class="timing-item">
                            <div class="timing-label">Status</div>
                            <div class="timing-value{' violated' if header['status'] == 'VIOLATED' else ''}">{header['status']}</div>
                        </div>
                        <div class="timing-item">
                            <div class="timing-label">Slack</div>
                            <div class="timing-value">{header['slack']:.3f} ns</div>
                        </div>
                        <div class="timing-item">
                            <div class="timing-label">Total Delay</div>
                            <div class="timing-value">{header['data_path_delay']['total']:.3f} ns</div>
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
            <div class="chart-container">
                <canvas id="chart-path-{i}"></canvas>
            </div>
            <div class="segment-details" id="details-path-{i}">
                <h4>Segment Details</h4>
                <div id="details-content-path-{i}"></div>
            </div>
        </div>
'''

    # Add the JavaScript section
    html_template += '''
    </div>

    <script>
        // Data for all paths (preserving critical path order)
        const pathsData = ''' + json.dumps(paths_data) + ''';
        const colorMap = ''' + json.dumps(color_map) + ''';
        const resourceTypes = ''' + json.dumps(resource_types) + ''';
        const targetTiming = ''' + str(target_timing) + '''; // Target timing threshold for red zone
        
        let disabledTypes = new Set();
        let charts = [];
        let selectedSegments = new Map();

        // Initialize charts for each path
        function initializeCharts() {
            pathsData.forEach((pathData, pathIndex) => {
                const ctx = document.getElementById(`chart-path-${pathIndex}`).getContext('2d');
                
                // Create individual datasets for each segment IN ORDER
                const datasets = pathData.segments.map((segment, segmentIndex) => ({
                    label: `${segment.kind}-${segmentIndex}`, // Unique label for each segment
                    data: [segment.delay],
                    backgroundColor: colorMap[segment.kind] || '#666',
                    borderColor: '#2c3e50',
                    borderWidth: 1,
                    segment: segment,
                    hidden: disabledTypes.has(segment.kind),
                    resourceType: segment.kind,
                    sequenceIndex: segmentIndex
                }));
                
                const chart = new Chart(ctx, {
                    type: 'bar',
                    data: {
                        labels: [`Path ${pathIndex + 1}`],
                        datasets: datasets
                    },
                    options: {
                        indexAxis: 'y',
                        responsive: true,
                        maintainAspectRatio: false,
                        interaction: {
                            mode: 'nearest',
                            intersect: false
                        },
                        scales: {
                            x: {
                                stacked: true,
                                beginAtZero: true,
                                title: {
                                    display: true,
                                    text: 'Delay (ns)'
                                },
                                ticks: {
                                    callback: function(value) {
                                        return value.toFixed(3) + ' ns';
                                    },
                                    color: function(context) {
                                        // Color x-axis values dark red if they exceed target timing
                                        return context.tick.value >= targetTiming ? '#c0392b' : '#2c3e50';
                                    }
                                }
                            },
                            y: {
                                stacked: true,
                                display: false  // Hide y-axis labels
                            }
                        },
                        plugins: {
                            legend: {
                                display: false  // We have our own legend
                            },
                            tooltip: {
                                mode: 'nearest',
                                intersect: false,
                                backgroundColor: 'rgba(0, 0, 0, 0.9)',
                                titleColor: 'white',
                                bodyColor: 'white',
                                borderColor: 'rgba(255, 255, 255, 0.3)',
                                borderWidth: 1,
                                displayColors: false,
                                callbacks: {
                                    title: function(context) {
                                        const dataset = context[0].dataset;
                                        const segment = dataset.segment;
                                        return `${segment.kind} (Sequence ${dataset.sequenceIndex + 1})`;
                                    },
                                    label: function(context) {
                                        const segment = context.dataset.segment;
                                        let tooltipText = [`Delay: ${context.parsed.x.toFixed(3)} ns`];
                                        tooltipText.push(`Resource: ${segment.resource_name}`);
                                        tooltipText.push(`Location: ${segment.location}`);
                                        if (segment.fan_out > 0) {
                                            tooltipText.push(`Fan-out: ${segment.fan_out}`);
                                        }
                                        if (segment.net_name) {
                                            tooltipText.push(`Net: ${segment.net_name}`);
                                        }
                                        return tooltipText;
                                    }
                                }
                            },
                            annotation: {
                                annotations: {
                                    violationZone: {
                                        type: 'box',
                                        xMin: targetTiming,
                                        xMax: 'max', // Extend to end of chart
                                        backgroundColor: 'rgba(231, 76, 60, 0.15)', // Slightly darker red for better visibility
                                        borderWidth: 0 // No border for minimal approach
                                    }
                                }
                            }
                        },
                        onHover: (event, elements) => {
                            // Debug logging for hover events
                            console.log(`Hover on chart ${pathIndex}: ${elements.length} elements found`);
                            if (elements.length > 0) {
                                console.log('Element details:', elements[0]);
                            }
                        },
                        onClick: (event, elements) => {
                            if (elements.length > 0) {
                                const elementIndex = elements[0].index;
                                const datasetIndex = elements[0].datasetIndex;
                                const dataset = chart.data.datasets[datasetIndex];
                                showSegmentDetails(pathIndex, dataset);
                            }
                        },
                        animation: {
                            duration: 300,
                            easing: 'easeOutQuart'
                        }
                    }
                });
                
                charts.push(chart);
            });
        }

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
                
                updateChartsVisibility();
            });
        });

        function updateChartsVisibility() {
            charts.forEach(chart => {
                chart.data.datasets.forEach(dataset => {
                    dataset.hidden = disabledTypes.has(dataset.resourceType);
                });
                chart.update('none'); // Update without animation for better performance
            });
        }

        function showSegmentDetails(pathIndex, dataset) {
            const detailsDiv = document.getElementById(`details-path-${pathIndex}`);
            const contentDiv = document.getElementById(`details-content-path-${pathIndex}`);
            const segment = dataset.segment;
            
            const isCurrentlySelected = selectedSegments.get(pathIndex) === dataset.label;
            
            if (isCurrentlySelected) {
                detailsDiv.classList.remove('show');
                selectedSegments.delete(pathIndex);
            } else {
                selectedSegments.set(pathIndex, dataset.label);
                
                const detailsContent = `
                    <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px;">
                        <div>
                            <h5>Resource Information</h5>
                            <p><strong>Type:</strong> ${segment.kind}</p>
                            <p><strong>Name:</strong> ${segment.resource_name}</p>
                            <p><strong>Location:</strong> ${segment.location}</p>
                            <p><strong>Sequence:</strong> ${segment.sequenceIndex + 1} of ${pathsData[pathIndex].segments.length}</p>
                        </div>
                        <div>
                            <h5>Timing Information</h5>
                            <p><strong>Delay:</strong> ${segment.delay.toFixed(3)} ns</p>
                            <p><strong>Delay Type:</strong> ${segment.delay_type}</p>
                        </div>
                        <div>
                            <h5>Connectivity</h5>
                            <p><strong>Fan-out:</strong> ${segment.fan_out}</p>
                            <p><strong>Net Name:</strong> ${segment.net_name || 'N/A'}</p>
                        </div>
                    </div>
                `;
                
                contentDiv.innerHTML = detailsContent;
                detailsDiv.classList.add('show');
            }
        }


        // Initialize everything
        document.addEventListener('DOMContentLoaded', function() {
            initializeCharts();
        });
    </script>
</body>
</html>'''

    with open(output_file, 'w') as f:
        f.write(html_template)

def main():
    if len(sys.argv) < 2:
        print("Usage: python visualize_timing_chartjs_clean.py <input.rpt> [output.html]")
        sys.exit(1)
    
    rpt_file = sys.argv[1]
    output_file = sys.argv[2] if len(sys.argv) > 2 else 'timing_visualization_chartjs_clean.html'
    
    if not Path(rpt_file).exists():
        print(f"Error: Input file '{rpt_file}' not found")
        sys.exit(1)
    
    print(f"Parsing RPT file: {rpt_file}")
    critical_paths = parse_rpt_file(rpt_file)
    
    print(f"Found {len(critical_paths)} critical paths")
    print(f"Generating Clean Sequential Chart.js HTML visualization: {output_file}")
    
    generate_html(critical_paths, output_file)
    print(f"Clean sequential Chart.js visualization saved to {output_file}")
    print("\nFEATURES:")
    print("✅ Segments appear in EXACT critical path order")
    print("✅ Individual Chart.js datasets for each segment")
    print("✅ Professional styling and animations")
    print("✅ LUT color family gradients")
    print("✅ Interactive tooltips and details")
    print("✅ Resource type filtering")
    print("✅ Target frequency red zone highlighting timing violations")

if __name__ == "__main__":
    main()