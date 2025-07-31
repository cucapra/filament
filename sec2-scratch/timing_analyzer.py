#!/usr/bin/env python3
"""
Vivado Timing Report Parser and Visualizer
Generates interactive HTML pages with critical path waterfall charts
and physical layout heatmaps
"""

import re
import json
import sys
import argparse
from pathlib import Path
from typing import Dict, List
import logging

# HTML template for interactive visualization
HTML_TEMPLATE = """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Timing Analysis - {design_name}</title>
    <script src="https://cdn.plot.ly/plotly-latest.min.js"></script>
    <script src="https://d3js.org/d3.v7.min.js"></script>
    <style>
        body {{
            font-family: Arial, sans-serif;
            margin: 20px;
            background-color: #f5f5f5;
        }}
        .container {{
            max-width: 1400px;
            margin: 0 auto;
            background-color: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
            padding: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border-radius: 8px;
        }}
        .summary {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin-bottom: 30px;
        }}
        .metric {{
            text-align: center;
            padding: 15px;
            border-radius: 8px;
            background-color: #f8f9fa;
            border-left: 4px solid #007bff;
        }}
        .metric.warning {{
            border-left-color: #ffc107;
            background-color: #fff8e1;
        }}
        .metric.danger {{
            border-left-color: #dc3545;
            background-color: #ffebee;
        }}
        .metric-value {{
            font-size: 24px;
            font-weight: bold;
            margin-bottom: 5px;
        }}
        .metric-label {{
            font-size: 12px;
            color: #666;
            text-transform: uppercase;
        }}
        .section {{
            margin: 30px 0;
        }}
        .section-title {{
            font-size: 20px;
            font-weight: bold;
            margin-bottom: 15px;
            color: #333;
            border-bottom: 2px solid #eee;
            padding-bottom: 10px;
        }}
        .path-selector {{
            margin: 20px 0;
            padding: 15px;
            background-color: #f8f9fa;
            border-radius: 8px;
            display: flex;
            align-items: center;
            gap: 15px;
            flex-wrap: wrap;
        }}
        .path-selector label {{
            font-weight: 500;
            color: #333;
            display: flex;
            align-items: center;
            gap: 8px;
        }}
        .path-selector select {{
            width: 300px;
            padding: 10px;
            border: 1px solid #ddd;
            border-radius: 4px;
            font-size: 14px;
        }}
        .path-selector input[type="checkbox"] {{
            margin-right: 5px;
            transform: scale(1.2);
        }}
        .chart-container {{
            background-color: white;
            border: 1px solid #ddd;
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
        }}
        .tooltip {{
            position: absolute;
            background: rgba(0,0,0,0.8);
            color: white;
            padding: 10px;
            border-radius: 4px;
            font-size: 12px;
            pointer-events: none;
            z-index: 1000;
        }}
        .legend {{
            display: flex;
            justify-content: center;
            margin: 10px 0;
            flex-wrap: wrap;
        }}
        .legend-item {{
            display: flex;
            align-items: center;
            margin: 5px 15px;
            font-size: 12px;
        }}
        .legend-color {{
            width: 15px;
            height: 15px;
            margin-right: 5px;
            border-radius: 2px;
        }}

        /* Resource Legend Styles */
        .resource-legend {{
            margin-top: 20px;
            padding: 15px;
            background: #f8f9fa;
            border-radius: 8px;
            border: 1px solid #e9ecef;
        }}
        .resource-legend h4 {{
            margin: 0 0 15px 0;
            color: #333;
            font-size: 16px;
            font-weight: 600;
        }}
        .resource-legend .legend-items {{
            display: flex;
            flex-wrap: wrap;
            gap: 15px;
        }}
        .resource-legend .legend-item {{
            display: flex;
            align-items: center;
            gap: 8px;
            padding: 8px 12px;
            background: white;
            border: 1px solid #dee2e6;
            border-radius: 6px;
            font-size: 14px;
            font-weight: 500;
            color: #495057;
        }}
        .resource-legend .legend-color {{
            width: 16px;
            height: 16px;
            border-radius: 3px;
            border: 1px solid rgba(0,0,0,0.1);
        }}
        .grid {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 20px;
        }}
        @media (max-width: 768px) {{
            .grid {{
                grid-template-columns: 1fr;
            }}
            .summary {{
                grid-template-columns: 1fr;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Timing Analysis Report</h1>
            <h2>{design_name}</h2>
            <p>Generated from: {report_file}</p>
        </div>

        <div class="summary">
            <div class="metric {wns_class}">
                <div class="metric-value">{wns}ns</div>
                <div class="metric-label">Worst Negative Slack</div>
            </div>
            <div class="metric {tns_class}">
                <div class="metric-value">{tns}ns</div>
                <div class="metric-label">Total Negative Slack</div>
            </div>
            <div class="metric {whs_class}">
                <div class="metric-value">{whs}ns</div>
                <div class="metric-label">Worst Hold Slack</div>
            </div>
            <div class="metric">
                <div class="metric-value">{freq_mhz}MHz</div>
                <div class="metric-label">Max Frequency</div>
            </div>
            <div class="metric {violations_class}">
                <div class="metric-value">{failing_endpoints}</div>
                <div class="metric-label">Failing Endpoints</div>
            </div>
            <div class="metric">
                <div class="metric-value">{total_endpoints}</div>
                <div class="metric-label">Total Endpoints</div>
            </div>
        </div>

        <div class="section">
            <div class="section-title">Critical Path Analysis</div>
            <div class="path-selector">
                <label for="pathSelect">Select Critical Path:</label>
                <select id="pathSelect" onchange="updatePath()">
                    <!-- Options will be populated by JavaScript -->
                </select>
                <label for="showRouting" style="margin-left: 20px;">
                    <input type="checkbox" id="showRouting" checked onchange="updatePath()">
                    Include Routing Delays
                </label>
            </div>
            <div class="chart-container">
                <div id="waterfallChart"></div>
            </div>
        </div>

        <div class="grid">
            <div class="section">
                <div class="section-title">Physical Layout Heatmap</div>
                <div class="chart-container">
                    <div id="heatmapChart"></div>
                </div>
                <div class="legend">
                    <div class="legend-item">
                        <div class="legend-color" style="background-color: #2E8B57;"></div>
                        <span>Fast (&lt;1ns)</span>
                    </div>
                    <div class="legend-item">
                        <div class="legend-color" style="background-color: #FFD700;"></div>
                        <span>Medium (1-2ns)</span>
                    </div>
                    <div class="legend-item">
                        <div class="legend-color" style="background-color: #FF6347;"></div>
                        <span>Slow (&gt;2ns)</span>
                    </div>
                </div>
            </div>

            <div class="section">
                <div class="section-title">Slack Distribution</div>
                <div class="chart-container">
                    <div id="slackHistogram"></div>
                </div>
            </div>
        </div>

        <div class="section">
            <div class="section-title">Logic Type Analysis</div>
            <div class="chart-container">
                <div id="logicChart"></div>
            </div>
        </div>
    </div>

    <script>
        // Data from Python
        const timingData = {timing_data_json};

        let currentPathIndex = 0;

        // Populate path selector
        function populatePathSelector() {{
            const select = document.getElementById('pathSelect');
            timingData.critical_paths.forEach((path, index) => {{
                const option = document.createElement('option');
                option.value = index;
                option.textContent = `Path ${{index + 1}}: ${{path.destination}} (Slack: ${{path.slack_ns}}ns)`;
                select.appendChild(option);
            }});
        }}

        // Create horizontal bar chart for critical path
        function createWaterfallChart(pathIndex) {{
            const path = timingData.critical_paths[pathIndex];
            if (!path || !path.stages) {{
                console.warn('No path data available for index:', pathIndex);
                return;
            }}

            const stages = path.stages;
            const showRouting = document.getElementById('showRouting').checked;

            // Filter stages based on routing toggle
            const filteredStages = showRouting ? stages : stages.filter(stage =>
                !stage.element.toLowerCase().includes('net') &&
                stage.resource_type !== 'ROUTING'
            );

            // Calculate cumulative positions for horizontal bar regions
            let cumulativeDelay = 0;
            const regions = filteredStages.map((stage, i) => {{
                const startPos = cumulativeDelay;
                cumulativeDelay += stage.delay_ns;
                const endPos = cumulativeDelay;

                return {{
                    ...stage,
                    startPos,
                    endPos,
                    width: stage.delay_ns,
                    index: i
                }};
            }});

            // Clear existing chart
            const chartContainer = document.getElementById('waterfallChart');
            chartContainer.innerHTML = '';

            // Create custom SVG-based chart
            createCustomBarChart(chartContainer, regions, path);

            // Create resource type legend
            createResourceLegend(filteredStages);
        }}

        // Create custom interactive bar chart
        function createCustomBarChart(container, regions, path) {{
            const margin = {{ top: 60, right: 30, bottom: 80, left: 60 }};
            const width = container.offsetWidth - margin.left - margin.right;
            const height = 200;
            const barHeight = 40;

            // Enhanced color mapping
            const resourceColors = {{
                'REGISTER': '#4CAF50',
                'LUT': '#2196F3',
                'CARRY': '#FF9800',
                'DSP': '#9C27B0',
                'BRAM': '#795548',
                'IO': '#607D8B',
                'ROUTING': '#F44336',
                'MUX': '#FFEB3B',
                'ADDER': '#00BCD4',
                'OTHER': '#9E9E9E'
            }};

            // Calculate scale
            const totalDelay = regions.length > 0 ? regions[regions.length - 1].endPos : 1;
            const xScale = width / totalDelay;

            // Create SVG
            const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
            svg.setAttribute('width', width + margin.left + margin.right);
            svg.setAttribute('height', height + margin.top + margin.bottom);
            svg.style.cssText = 'background: #f8f9fa; border-radius: 8px;';

            // Create main group
            const g = document.createElementNS('http://www.w3.org/2000/svg', 'g');
            g.setAttribute('transform', `translate(${{margin.left}},${{margin.top}})`);
            svg.appendChild(g);

            // Add title
            const title = document.createElementNS('http://www.w3.org/2000/svg', 'text');
            title.setAttribute('x', width / 2);
            title.setAttribute('y', -30);
            title.setAttribute('text-anchor', 'middle');
            title.setAttribute('font-size', '16');
            title.setAttribute('font-weight', 'bold');
            title.setAttribute('fill', '#333');
            title.textContent = `Critical Path: ${{path.destination}} (Slack: ${{path.slack_ns}}ns)`;
            g.appendChild(title);

            // Add X axis
            const xAxis = document.createElementNS('http://www.w3.org/2000/svg', 'line');
            xAxis.setAttribute('x1', 0);
            xAxis.setAttribute('x2', width);
            xAxis.setAttribute('y1', height / 2 + barHeight / 2 + 10);
            xAxis.setAttribute('y2', height / 2 + barHeight / 2 + 10);
            xAxis.setAttribute('stroke', '#666');
            xAxis.setAttribute('stroke-width', 1);
            g.appendChild(xAxis);

            // Add axis label
            const axisLabel = document.createElementNS('http://www.w3.org/2000/svg', 'text');
            axisLabel.setAttribute('x', width / 2);
            axisLabel.setAttribute('y', height / 2 + barHeight / 2 + 40);
            axisLabel.setAttribute('text-anchor', 'middle');
            axisLabel.setAttribute('font-size', '14');
            axisLabel.setAttribute('fill', '#666');
            axisLabel.textContent = 'Cumulative Delay (ns)';
            g.appendChild(axisLabel);

            // Add tick marks and labels
            const numTicks = 5;
            for (let i = 0; i <= numTicks; i++) {{
                const value = (totalDelay / numTicks) * i;
                const x = (width / numTicks) * i;

                // Tick mark
                const tick = document.createElementNS('http://www.w3.org/2000/svg', 'line');
                tick.setAttribute('x1', x);
                tick.setAttribute('x2', x);
                tick.setAttribute('y1', height / 2 + barHeight / 2 + 10);
                tick.setAttribute('y2', height / 2 + barHeight / 2 + 15);
                tick.setAttribute('stroke', '#666');
                g.appendChild(tick);

                // Tick label
                const tickLabel = document.createElementNS('http://www.w3.org/2000/svg', 'text');
                tickLabel.setAttribute('x', x);
                tickLabel.setAttribute('y', height / 2 + barHeight / 2 + 30);
                tickLabel.setAttribute('text-anchor', 'middle');
                tickLabel.setAttribute('font-size', '12');
                tickLabel.setAttribute('fill', '#666');
                tickLabel.textContent = value.toFixed(2);
                g.appendChild(tickLabel);
            }}

            // Create regions
            const activeTooltips = new Set();

            regions.forEach((region, i) => {{
                const x = region.startPos * xScale;
                const regionWidth = region.width * xScale;
                const y = height / 2 - barHeight / 2;
                const resourceType = region.resource_type || 'OTHER';
                const color = resourceColors[resourceType] || resourceColors['OTHER'];

                // Create region group
                const regionGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
                regionGroup.setAttribute('class', 'region-group');
                regionGroup.setAttribute('data-index', i);

                // Create main rectangle
                const rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
                rect.setAttribute('x', x);
                rect.setAttribute('y', y);
                rect.setAttribute('width', regionWidth);
                rect.setAttribute('height', barHeight);
                rect.setAttribute('fill', color);
                rect.setAttribute('stroke', '#333');
                rect.setAttribute('stroke-width', 1);
                rect.setAttribute('rx', 3);
                rect.style.cssText = 'cursor: pointer; transition: all 0.3s cubic-bezier(0.68, -0.55, 0.265, 1.55);';

                // Add hover and click interactions
                let tooltip = null;
                let isClicked = false;

                function createTooltip() {{
                    if (tooltip) return tooltip;

                    const tooltipGroup = document.createElementNS('http://www.w3.org/2000/svg', 'g');
                    tooltipGroup.setAttribute('class', 'tooltip-group');

                    // Tooltip background
                    const tooltipBg = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
                    tooltipBg.setAttribute('fill', 'rgba(0,0,0,0.9)');
                    tooltipBg.setAttribute('rx', 8);
                    tooltipBg.setAttribute('stroke', color);
                    tooltipBg.setAttribute('stroke-width', 2);

                    // Tooltip text
                    const tooltipText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
                    tooltipText.setAttribute('fill', 'white');
                    tooltipText.setAttribute('font-size', '12');
                    tooltipText.setAttribute('font-family', 'Arial, sans-serif');

                    // Build tooltip content
                    const lines = [];
                    lines.push(region.element);
                    lines.push(`Delay: ${{region.delay_ns.toFixed(3)}}ns`);
                    lines.push(`Position: ${{region.startPos.toFixed(3)}} - ${{region.endPos.toFixed(3)}}ns`);
                    lines.push(`Location: ${{region.location || 'N/A'}}`);
                    if (region.netlist_resource) {{
                        const instanceName = region.hierarchy_parts?.instance_name || region.netlist_resource;
                        lines.push(`Resource: ${{instanceName}}`);
                    }}
                    if (region.hierarchy_parts?.top_module) {{
                        lines.push(`Module: ${{region.hierarchy_parts.top_module}}`);
                    }}

                    // Add text lines
                    lines.forEach((line, lineIndex) => {{
                        const tspan = document.createElementNS('http://www.w3.org/2000/svg', 'tspan');
                        tspan.setAttribute('x', 12);
                        tspan.setAttribute('dy', lineIndex === 0 ? 15 : 16);
                        if (lineIndex === 0) tspan.setAttribute('font-weight', 'bold');
                        tspan.textContent = line;
                        tooltipText.appendChild(tspan);
                    }});

                    // Calculate tooltip size and position
                    tooltipGroup.appendChild(tooltipText);
                    svg.appendChild(tooltipGroup);
                    const bbox = tooltipText.getBBox();

                    tooltipBg.setAttribute('width', bbox.width + 24);
                    tooltipBg.setAttribute('height', bbox.height + 20);

                    // Position tooltip closer to the region and within bounds
                    const tooltipX = Math.max(10, Math.min(width - bbox.width - 34, x + regionWidth / 2 - bbox.width / 2 - 12));
                    const tooltipY = Math.max(10, y - bbox.height - 15);

                    tooltipGroup.setAttribute('transform', `translate(${{tooltipX}}, ${{tooltipY}})`);
                    tooltipGroup.insertBefore(tooltipBg, tooltipText);

                    // Add close button for clicked tooltips
                    const closeBtn = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
                    closeBtn.setAttribute('cx', bbox.width + 24 - 8);
                    closeBtn.setAttribute('cy', 8);
                    closeBtn.setAttribute('r', 8);
                    closeBtn.setAttribute('fill', '#ff4444');
                    closeBtn.setAttribute('stroke', 'white');
                    closeBtn.setAttribute('stroke-width', 1);
                    closeBtn.style.cssText = 'cursor: pointer; opacity: 0;';

                    const closeX = document.createElementNS('http://www.w3.org/2000/svg', 'text');
                    closeX.setAttribute('x', bbox.width + 24 - 8);
                    closeX.setAttribute('y', 12);
                    closeX.setAttribute('text-anchor', 'middle');
                    closeX.setAttribute('fill', 'white');
                    closeX.setAttribute('font-size', '10');
                    closeX.setAttribute('font-weight', 'bold');
                    closeX.textContent = '×';
                    closeX.style.cssText = 'cursor: pointer; opacity: 0; pointer-events: none;';

                    tooltipGroup.appendChild(closeBtn);
                    tooltipGroup.appendChild(closeX);

                    // Close button functionality
                    closeBtn.addEventListener('click', (e) => {{
                        e.stopPropagation();
                        hideTooltip();
                    }});

                    // Animation
                    tooltipGroup.style.cssText = 'opacity: 0; transform-origin: 50% 100%;';
                    requestAnimationFrame(() => {{
                        tooltipGroup.style.cssText = 'opacity: 1; transition: all 0.3s cubic-bezier(0.68, -0.55, 0.265, 1.55);';
                    }});

                    tooltip = tooltipGroup;
                    return tooltip;
                }}

                function showTooltip() {{
                    createTooltip();
                    // Animate region bounce (reduced magnitude)
                    rect.style.transform = 'scaleY(1.05) translateY(-1px)';
                    rect.style.filter = 'drop-shadow(0 2px 4px rgba(0,0,0,0.2))';
                }}

                function hideTooltip() {{
                    if (tooltip && !isClicked) {{
                        tooltip.style.opacity = '0';
                        setTimeout(() => {{
                            if (tooltip && tooltip.parentNode) {{
                                tooltip.parentNode.removeChild(tooltip);
                            }}
                            tooltip = null;
                        }}, 300);
                    }}
                    if (!isClicked) {{
                        rect.style.transform = '';
                        rect.style.filter = '';
                    }}
                }}

                function toggleClick() {{
                    isClicked = !isClicked;

                    if (isClicked) {{
                        activeTooltips.add(i);
                        showTooltip();
                        if (tooltip) {{
                            const closeBtn = tooltip.querySelector('circle');
                            const closeX = tooltip.querySelector('text:last-child');
                            if (closeBtn) closeBtn.style.opacity = '1';
                            if (closeX) closeX.style.opacity = '1';
                        }}
                    }} else {{
                        activeTooltips.delete(i);
                        hideTooltip();
                    }}
                }}

                // Event listeners
                rect.addEventListener('mouseenter', showTooltip);
                rect.addEventListener('mouseleave', hideTooltip);
                rect.addEventListener('click', toggleClick);

                regionGroup.appendChild(rect);
                g.appendChild(regionGroup);
            }});

            container.appendChild(svg);
        }}

        // Create legend for resource types
        function createResourceLegend(stages) {{
            const resourceColors = {{
                'REGISTER': '#4CAF50',
                'LUT': '#2196F3',
                'CARRY': '#FF9800',
                'DSP': '#9C27B0',
                'BRAM': '#795548',
                'IO': '#607D8B',
                'ROUTING': '#F44336',
                'MUX': '#FFEB3B',
                'ADDER': '#00BCD4',
                'OTHER': '#9E9E9E'
            }};

            // Get unique resource types from current path
            const usedTypes = [...new Set(stages.map(stage => stage.resource_type || 'OTHER'))];

            // Create legend HTML
            let legendHtml = '<div class="resource-legend"><h4>Resource Types in Path:</h4><div class="legend-items">';
            usedTypes.forEach(type => {{
                const color = resourceColors[type];
                const displayName = type.replace('_', ' ').toLowerCase().replace(/\\b\\w/g, l => l.toUpperCase());
                legendHtml += `<div class="legend-item">
                    <div class="legend-color" style="background-color: ${{color}};"></div>
                    <span>${{displayName}}</span>
                </div>`;
            }});
            legendHtml += '</div></div>';

            // Add legend after the chart
            const chartContainer = document.getElementById('waterfallChart').parentElement;
            let existingLegend = chartContainer.querySelector('.resource-legend');
            if (existingLegend) {{
                existingLegend.remove();
            }}
            chartContainer.insertAdjacentHTML('beforeend', legendHtml);
        }}

        // Create physical layout heatmap
        function createHeatmap() {{
            if (!timingData.physical_layout || timingData.physical_layout.length === 0) {{
                document.getElementById('heatmapChart').innerHTML = '<p>No physical layout data available</p>';
                return;
            }}

            const layout_data = timingData.physical_layout;
            const x_coords = [...new Set(layout_data.map(d => d.x))].sort((a, b) => a - b);
            const y_coords = [...new Set(layout_data.map(d => d.y))].sort((a, b) => a - b);

            // Create delay matrix
            const z = y_coords.map(y =>
                x_coords.map(x => {{
                    const point = layout_data.find(d => d.x === x && d.y === y);
                    return point ? point.delay_ns : 0;
                }})
            );

            const trace = {{
                z: z,
                x: x_coords,
                y: y_coords,
                type: 'heatmap',
                colorscale: [
                    [0, '#2E8B57'],    // Green for fast
                    [0.5, '#FFD700'],  // Yellow for medium
                    [1, '#FF6347']     // Red for slow
                ],
                hoverongaps: false,
                hovertemplate: 'X: %{{x}}<br>Y: %{{y}}<br>Delay: %{{z:.3f}}ns<extra></extra>'
            }};

            const layout = {{
                title: 'FPGA Physical Layout - Timing Delays',
                xaxis: {{ title: 'X Coordinate' }},
                yaxis: {{ title: 'Y Coordinate' }},
                margin: {{ l: 60, r: 30, t: 60, b: 60 }}
            }};

            Plotly.newPlot('heatmapChart', [trace], layout, {{responsive: true}});
        }}

        // Create slack distribution histogram
        function createSlackHistogram() {{
            const slacks = timingData.critical_paths.map(path => path.slack_ns);

            const trace = {{
                x: slacks,
                type: 'histogram',
                nbinsx: 20,
                marker: {{
                    color: '#2196F3',
                    line: {{
                        width: 1,
                        color: '#333'
                    }}
                }}
            }};

            const layout = {{
                title: 'Slack Distribution',
                xaxis: {{ title: 'Slack (ns)' }},
                yaxis: {{ title: 'Number of Paths' }},
                margin: {{ l: 60, r: 30, t: 60, b: 60 }}
            }};

            Plotly.newPlot('slackHistogram', [trace], layout, {{responsive: true}});
        }}

        // Create logic type analysis chart
        function createLogicChart() {{
            const logicTypes = {{}};
            timingData.critical_paths.forEach(path => {{
                if (path.stages) {{
                    path.stages.forEach(stage => {{
                        const element = stage.element;
                        if (element) {{
                            logicTypes[element] = (logicTypes[element] || 0) + 1;
                        }}
                    }});
                }}
            }});

            const labels = Object.keys(logicTypes);
            const values = Object.values(logicTypes);

            const trace = {{
                labels: labels,
                values: values,
                type: 'pie',
                textinfo: 'label+percent',
                textposition: 'outside',
                marker: {{
                    colors: ['#FF6384', '#36A2EB', '#FFCE56', '#4BC0C0', '#9966FF', '#FF9F40']
                }}
            }};

            const layout = {{
                title: 'Logic Element Distribution',
                margin: {{ l: 60, r: 60, t: 60, b: 60 }}
            }};

            Plotly.newPlot('logicChart', [trace], layout, {{responsive: true}});
        }}

        // Update path display
        function updatePath() {{
            const select = document.getElementById('pathSelect');
            currentPathIndex = parseInt(select.value);
            createWaterfallChart(currentPathIndex);
        }}

        // Initialize all charts
        function initializeCharts() {{
            populatePathSelector();
            createWaterfallChart(0);
            createHeatmap();
            createSlackHistogram();
            createLogicChart();
        }}

        // Initialize when page loads
        document.addEventListener('DOMContentLoaded', initializeCharts);
    </script>
</body>
</html>
"""


class TimingReportParser:
    def __init__(self):
        self.logger = logging.getLogger(__name__)

    def parse_report(self, report_path: Path) -> Dict:
        """Parse a Vivado timing report file"""
        try:
            with open(report_path, "r", encoding="utf-8", errors="ignore") as f:
                content = f.read()
        except Exception as e:
            self.logger.error(f"Failed to read {report_path}: {e}")
            return {}

        result = {
            "design_name": self._extract_design_name(content),
            "summary": self._extract_summary(content),
            "critical_paths": self._extract_critical_paths(content),
            "physical_layout": self._extract_physical_layout(content),
            "report_file": str(report_path),
        }

        return result

    def _extract_design_name(self, content: str) -> str:
        """Extract design name from report"""
        match = re.search(r"Design\s*:\s*(\w+)", content)
        return match.group(1) if match else "Unknown"

    def _extract_summary(self, content: str) -> Dict:
        """Extract timing summary information"""
        summary = {}

        # Extract timing summary table - handle both numeric and 'inf' values
        summary_pattern = r"WNS\(ns\)\s+TNS\(ns\)\s+.*?\n\s*[-\s]+\n\s*([\d\.\-inf]+)\s+([\d\.\-inf]+)\s+(\d+)\s+(\d+)\s+([\d\.\-inf]+)\s+([\d\.\-inf]+)\s+(\d+)\s+(\d+)"
        match = re.search(summary_pattern, content)

        if match:
            # Handle 'inf' values by converting to large positive numbers
            def parse_value(val_str):
                if val_str.strip() == "inf":
                    return 999.0  # Large positive value for infinite slack
                else:
                    return float(val_str)

            summary.update(
                {
                    "wns_ns": parse_value(match.group(1)),
                    "tns_ns": parse_value(match.group(2)),
                    "tns_failing_endpoints": int(match.group(3)),
                    "tns_total_endpoints": int(match.group(4)),
                    "whs_ns": parse_value(match.group(5)),
                    "ths_ns": parse_value(match.group(6)),
                    "ths_failing_endpoints": int(match.group(7)),
                    "ths_total_endpoints": int(match.group(8)),
                }
            )

        # Calculate frequency
        if "wns_ns" in summary:
            clock_period = 7.0  # Default, try to extract from constraints
            period_match = re.search(r"period=([0-9.]+)ns", content)
            if period_match:
                clock_period = float(period_match.group(1))

            # Achievable frequency considering worst slack
            wns = summary["wns_ns"]
            if wns >= 999:  # Handle infinite slack case
                summary["frequency_mhz"] = 1000.0 / clock_period
            else:
                achievable_period = clock_period - wns
                summary["frequency_mhz"] = (
                    1000.0 / achievable_period if achievable_period > 0 else 0
                )
            summary["clock_period_ns"] = clock_period

        return summary

    def _extract_critical_paths(self, content: str) -> List[Dict]:
        """Extract critical path information"""
        paths = []

        # Find all slack entries
        slack_pattern = r"Slack \((MET|VIOLATED)\)\s*:\s*([\d\.\-]+)ns.*?Source:\s*([^\n]+).*?Destination:\s*([^\n]+).*?Data Path Delay:\s*([\d\.]+)ns.*?Logic Levels:\s*(\d+)"

        for match in re.finditer(slack_pattern, content, re.DOTALL):
            status = match.group(1)
            slack = float(match.group(2))
            source = match.group(3).strip()
            destination = match.group(4).strip()
            data_path_delay = float(match.group(5))
            logic_levels = int(match.group(6))

            # Extract detailed path stages
            stages = self._extract_path_stages(content, match.end())

            paths.append(
                {
                    "status": status,
                    "slack_ns": slack,
                    "source": source,
                    "destination": destination,
                    "data_path_delay_ns": data_path_delay,
                    "logic_levels": logic_levels,
                    "stages": stages,
                }
            )

        return paths

    def _extract_path_stages(self, content: str, start_pos: int) -> List[Dict]:
        """Extract individual stages of a timing path"""
        stages = []

        # Look for the detailed path table after the summary
        path_section = content[start_pos : start_pos + 4000]  # Increased search window

        # Find the start of the timing path table
        table_start = path_section.find("Location             Delay type")
        if table_start == -1:
            return stages

        # Extract everything after the table header
        table_content = path_section[table_start:]

        # Split into lines and process each timing path entry
        lines = table_content.split("\n")

        for i, line in enumerate(lines):
            # Skip header lines and separators
            if "Location" in line or "---" in line or not line.strip():
                continue

            # Look for lines with SLICE locations and timing info
            slice_match = re.search(
                r"(SLICE_X\d+Y\d+)\s+(FDRE|FDSE|LUT\d+|CARRY\d+|DSP\d+E\d+|BRAM\w+|IBUF|OBUF)",
                line,
            )
            if slice_match:
                location = slice_match.group(1)
                element = slice_match.group(2)

                # Look for the next line which should have delay and netlist resource
                if i + 1 < len(lines):
                    next_line = lines[i + 1].strip()
                    delay_match = re.search(
                        r"([\d\.]+)\s+([\d\.]+)\s+[rf]\s+(.+)", next_line
                    )
                    if delay_match:
                        delay = float(delay_match.group(1))
                        cumulative = float(delay_match.group(2))
                        netlist_resource = delay_match.group(3).strip()

                        # Extract hierarchy components from netlist resource
                        hierarchy_parts = self._parse_netlist_hierarchy(
                            netlist_resource
                        )

                        stages.append(
                            {
                                "location": location,
                                "element": element,
                                "delay_ns": delay,
                                "cumulative_ns": cumulative,
                                "netlist_resource": netlist_resource,
                                "hierarchy_parts": hierarchy_parts,
                                "resource_type": self._classify_resource_type(
                                    element, netlist_resource
                                ),
                            }
                        )

            # Also look for net routing delays
            elif "net (fo=" in line:
                net_match = re.search(
                    r"net \(fo=\d+[^)]*\)\s+([\d\.]+)\s+([\d\.]+)\s+(.+)", line
                )
                if net_match:
                    delay = float(net_match.group(1))
                    cumulative = float(net_match.group(2))
                    netlist_resource = net_match.group(3).strip()

                    stages.append(
                        {
                            "location": "routing",
                            "element": "NET",
                            "delay_ns": delay,
                            "cumulative_ns": cumulative,
                            "netlist_resource": netlist_resource,
                            "hierarchy_parts": self._parse_netlist_hierarchy(
                                netlist_resource
                            ),
                            "resource_type": "ROUTING",
                        }
                    )

        return stages

    def _parse_netlist_hierarchy(self, netlist_resource: str) -> Dict:
        """Parse netlist resource string into hierarchy components"""
        parts = {
            "full_name": netlist_resource,
            "top_module": "",
            "sub_modules": [],
            "instance_name": "",
            "port_name": "",
        }

        # Split by '/' to get hierarchy levels
        if "/" in netlist_resource:
            hierarchy = netlist_resource.split("/")
            parts["top_module"] = hierarchy[0] if hierarchy else ""
            parts["sub_modules"] = hierarchy[1:-1] if len(hierarchy) > 2 else []

            # Extract instance and port from last part
            last_part = hierarchy[-1] if hierarchy else netlist_resource
            if "[" in last_part and "]" in last_part:
                # Handle array indices like out_reg[0]
                base_name = last_part.split("[")[0]
                parts["instance_name"] = base_name
                parts["port_name"] = last_part
            else:
                parts["instance_name"] = last_part
                parts["port_name"] = last_part
        else:
            parts["instance_name"] = netlist_resource
            parts["port_name"] = netlist_resource

        return parts

    def _classify_resource_type(self, element: str, netlist_resource: str) -> str:
        """Classify the type of FPGA resource for color coding"""
        if element in ["FDRE", "FDSE", "FDCE", "FDPE"]:
            return "REGISTER"
        elif element.startswith("LUT"):
            return "LUT"
        elif element.startswith("CARRY"):
            return "CARRY"
        elif element.startswith("DSP"):
            return "DSP"
        elif element.startswith("BRAM"):
            return "BRAM"
        elif element in ["IBUF", "OBUF"]:
            return "IO"
        elif "mux" in netlist_resource.lower() or "MUX" in netlist_resource:
            return "MUX"
        elif "add" in netlist_resource.lower() or "ADD" in netlist_resource:
            return "ADDER"
        else:
            return "OTHER"

    def _extract_physical_layout(self, content: str) -> List[Dict]:
        """Extract physical layout information from timing paths"""
        layout_data = []

        # Extract SLICE locations and their associated delays
        slice_pattern = r"SLICE_X(\d+)Y(\d+)\s+(\w+)\s+\([^)]+\)\s+([\d\.]+)"

        for match in re.finditer(slice_pattern, content):
            x_coord = int(match.group(1))
            y_coord = int(match.group(2))
            element_type = match.group(3)
            delay = float(match.group(4))

            layout_data.append(
                {
                    "x": x_coord,
                    "y": y_coord,
                    "element_type": element_type,
                    "delay_ns": delay,
                }
            )

        return layout_data


def generate_html_report(timing_data: Dict, output_path: Path):
    """Generate interactive HTML report"""

    summary = timing_data.get("summary", {})

    # Determine status classes for metrics
    def get_status_class(value, thresholds):
        if value < thresholds[0]:
            return ""
        elif value < thresholds[1]:
            return "warning"
        else:
            return "danger"

    wns = summary.get("wns_ns", 0)
    tns = summary.get("tns_ns", 0)
    whs = summary.get("whs_ns", 0)
    failing_endpoints = summary.get("tns_failing_endpoints", 0)
    total_endpoints = summary.get("tns_total_endpoints", 1)

    # Format the HTML with data
    html_content = HTML_TEMPLATE.format(
        design_name=timing_data.get("design_name", "Unknown"),
        report_file=timing_data.get("report_file", ""),
        wns=f"{wns:.3f}",
        wns_class=get_status_class(
            -wns, [0.5, 0.1]
        ),  # Negative because WNS should be positive
        tns=f"{tns:.3f}",
        tns_class=get_status_class(abs(tns), [0.1, 1.0]),
        whs=f"{whs:.3f}",
        whs_class=get_status_class(
            -whs, [0.5, 0.1]
        ),  # Negative because WHS should be positive for hold
        freq_mhz=f"{summary.get('frequency_mhz', 0):.1f}",
        failing_endpoints=failing_endpoints,
        violations_class=get_status_class(
            failing_endpoints / total_endpoints, [0.05, 0.2]
        ),
        total_endpoints=total_endpoints,
        timing_data_json=json.dumps(timing_data, indent=2),
    )

    # Write HTML file
    with open(output_path, "w") as f:
        f.write(html_content)


def main():
    parser = argparse.ArgumentParser(
        description="Parse Vivado timing reports and generate interactive visualizations"
    )
    parser.add_argument(
        "--input-dir",
        "-i",
        type=str,
        default="sec2-scratch",
        help="Directory to search for timing reports",
    )
    parser.add_argument(
        "--output-dir",
        "-o",
        type=str,
        default="timing_reports",
        help="Directory to output HTML reports",
    )
    parser.add_argument(
        "--report-pattern",
        "-p",
        type=str,
        default="*timing*summary*.rpt",
        help="Pattern to match timing report files",
    )
    parser.add_argument(
        "--verbose", "-v", action="store_true", help="Enable verbose logging"
    )

    args = parser.parse_args()

    # Setup logging
    log_level = logging.DEBUG if args.verbose else logging.INFO
    logging.basicConfig(
        level=log_level, format="%(asctime)s - %(levelname)s - %(message)s"
    )
    logger = logging.getLogger(__name__)

    # Create output directory
    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)

    # Find all timing reports
    input_path = Path(args.input_dir)
    if not input_path.exists():
        logger.error(f"Input directory {input_path} does not exist")
        return 1

    report_files = list(input_path.rglob(args.report_pattern))

    if not report_files:
        logger.warning(
            f"No timing report files found matching pattern {args.report_pattern} in {input_path}"
        )
        return 1

    logger.info(f"Found {len(report_files)} timing report files")

    # Parse each report and generate HTML
    parser_obj = TimingReportParser()

    for report_file in report_files:
        logger.info(f"Processing {report_file}")

        try:
            # Parse the timing report
            timing_data = parser_obj.parse_report(report_file)

            if not timing_data:
                logger.warning(f"Failed to parse {report_file}")
                continue

            # Generate output filename
            relative_path = report_file.relative_to(input_path)
            output_name = (
                str(relative_path)
                .replace("/", "_")
                .replace("\\", "_")
                .replace(".rpt", ".html")
            )
            output_file = output_dir / output_name

            # Generate HTML report
            generate_html_report(timing_data, output_file)

            logger.info(f"Generated {output_file}")

            # Print summary
            summary = timing_data.get("summary", {})
            logger.info(f"  Design: {timing_data.get('design_name', 'Unknown')}")
            logger.info(f"  WNS: {summary.get('wns_ns', 'N/A')}ns")
            logger.info(
                f"  Critical Paths: {len(timing_data.get('critical_paths', []))}"
            )
            logger.info(
                f"  Physical Layout Points: {len(timing_data.get('physical_layout', []))}"
            )

        except Exception as e:
            logger.error(f"Failed to process {report_file}: {e}")
            continue

    logger.info(f"Generated HTML reports in {output_dir}")
    logger.info("Open any .html file in a web browser to view the interactive analysis")

    return 0


if __name__ == "__main__":
    sys.exit(main())
