import re
import json
from datetime import datetime


class TimingReportParser:
    def __init__(self, filepath: str = None, content: str = None):
        """Initialize parser with either a file path or content string."""
        if filepath and content:
            raise ValueError("Provide either filepath or content, not both")
        if not filepath and not content:
            raise ValueError("Must provide either filepath or content")

        if filepath:
            with open(filepath, "r") as f:
                self.content = f.read()
        else:
            self.content = content

        self.parsed_data = None

    def parse(self) -> dict:
        """Parse the timing report and return structured data."""
        # Split content into sections
        sections = self._split_into_sections()

        # Parse each section
        design_info = self._parse_design_info(sections.get("header", ""))
        timing_summary = self._parse_timing_summary(sections.get("timing_summary", ""))
        clocks = self._parse_clock_summary(sections.get("clock_summary", ""))
        critical_paths = self._parse_critical_paths(sections.get("critical_paths", ""))

        self.parsed_data = {
            "design_info": design_info,
            "timing_summary": timing_summary,
            "clocks": clocks,
            "critical_paths": critical_paths,
        }

        return self.parsed_data

    def to_json(self, output_file: str = None, indent: int = 2) -> str:
        """Convert parsed data to JSON and optionally write to file."""
        if self.parsed_data is None:
            self.parse()

        json_str = json.dumps(self.parsed_data, indent=indent)

        if output_file:
            with open(output_file, "w") as f:
                f.write(json_str)

        return json_str

    def _split_into_sections(self) -> dict:
        """Split the report into major sections."""
        sections = {}

        # Find header (everything before "Timing Summary" or
        # "Design Timing Summary")
        timing_match = re.search(
            r"(Design Timing Summary|Timing Summary Report)", self.content
        )
        if timing_match:
            sections["header"] = self.content[: timing_match.start()]
            remaining = self.content[timing_match.start() :]
        else:
            sections["header"] = ""
            remaining = self.content

        # Find timing summary section
        timing_summary_match = re.search(
            r"Design Timing Summary.*?(?=Clock Summary|Max Delay Paths|$)",
            remaining,
            re.DOTALL,
        )
        if timing_summary_match:
            sections["timing_summary"] = timing_summary_match.group(0)

        # Find clock summary section
        clock_summary_match = re.search(
            r"Clock Summary.*?(?=Max Delay Paths|Min Delay Paths|"
            r"Pulse Width Checks|$)",
            remaining,
            re.DOTALL,
        )
        if clock_summary_match:
            sections["clock_summary"] = clock_summary_match.group(0)

        # Find critical paths (everything after timing/clock summary or direct timing data)
        paths_match = re.search(
            r"(Max Delay Paths|Min Delay Paths|Pulse Width Checks|Slack\s*\([^)]+\)|Slack\s*:\s*([-\d.]+|inf)).*",
            remaining,
            re.DOTALL,
        )
        if paths_match:
            sections["critical_paths"] = paths_match.group(0)

        return sections

    def _parse_design_info(self, header_text: str) -> dict:
        """Extract design information from the header section."""
        info = {}

        # Extract tool version
        tool_match = re.search(r"Tool Version.*?:\s*(.+)", header_text)
        if tool_match:
            info["tool_version"] = tool_match.group(1).strip()
        else:
            info["tool_version"] = "Unknown"

        # Extract date
        date_match = re.search(r"Date\s*:\s*(.+)", header_text)
        if date_match:
            date_str = date_match.group(1).strip()
            # Try to parse the date string and convert to ISO format
            try:
                # Handle format like "Wed Jul 30 16:41:44 2025"
                dt = datetime.strptime(date_str, "%a %b %d %H:%M:%S %Y")
                info["date"] = dt.isoformat()
            except ValueError:
                info["date"] = date_str
        else:
            info["date"] = datetime.now().isoformat()

        # Extract design name
        design_match = re.search(r"Design\s*:\s*(\w+)", header_text)
        if design_match:
            info["name"] = design_match.group(1)
        else:
            info["name"] = "unknown"

        # Extract device
        device_match = re.search(r"Device\s*:\s*([^\s]+)", header_text)
        if device_match:
            device = device_match.group(1)
            # Split device and speed grade
            parts = device.split("-")
            if len(parts) >= 2:
                info["device"] = "-".join(parts[:-1])
                info["speed_grade"] = "-" + parts[-1]
            else:
                info["device"] = device
                info["speed_grade"] = "-1"
        else:
            info["device"] = "unknown"
            info["speed_grade"] = "-1"

        # Extract design state
        state_match = re.search(r"Design State\s*:\s*(\w+)", header_text)
        if state_match:
            info["state"] = state_match.group(1)
        else:
            info["state"] = "Unknown"

        return info

    def _parse_timing_summary(self, summary_text: str) -> dict:
        """Parse the timing summary section for WNS, TNS, etc."""
        summary = {
            "setup": {
                "wns": 0.0,
                "tns": 0.0,
                "failing_endpoints": 0,
                "total_endpoints": 0,
            },
            "hold": {
                "wns": 0.0,
                "tns": 0.0,
                "failing_endpoints": 0,
                "total_endpoints": 0,
            },
            "pulse_width": {
                "wns": 0.0,
                "tns": 0.0,
                "failing_endpoints": 0,
                "total_endpoints": 0,
            },
        }

        # Look for the table with timing data
        table_match = re.search(
            r"WNS\(ns\).*?TNS.*?TNS Failing Endpoints.*?TNS Total Endpoints.*?WHS\(ns\).*?THS.*?THS Failing Endpoints.*?THS Total Endpoints.*?WPWS\(ns\).*?TPWS.*?TPWS Failing Endpoints.*?TPWS Total Endpoints.*?\n\s*[-\s]+\n\s*([^\n]+)",
            summary_text,
            re.DOTALL,
        )

        if table_match:
            values = table_match.group(1).split()
            if len(values) >= 12:
                # Setup timing (WNS, TNS, Failing, Total)
                summary["setup"]["wns"] = self._extract_numeric(values[0], 0.0)
                summary["setup"]["tns"] = self._extract_numeric(values[1], 0.0)
                summary["setup"]["failing_endpoints"] = int(
                    self._extract_numeric(values[2], 0)
                )
                summary["setup"]["total_endpoints"] = int(
                    self._extract_numeric(values[3], 0)
                )

                # Hold timing (WHS, THS, Failing, Total)
                summary["hold"]["wns"] = self._extract_numeric(values[4], 0.0)
                summary["hold"]["tns"] = self._extract_numeric(values[5], 0.0)
                summary["hold"]["failing_endpoints"] = int(
                    self._extract_numeric(values[6], 0)
                )
                summary["hold"]["total_endpoints"] = int(
                    self._extract_numeric(values[7], 0)
                )

                # Pulse width (WPWS, TPWS, Failing, Total)
                summary["pulse_width"]["wns"] = self._extract_numeric(values[8], 0.0)
                summary["pulse_width"]["tns"] = self._extract_numeric(values[9], 0.0)
                summary["pulse_width"]["failing_endpoints"] = int(
                    self._extract_numeric(values[10], 0)
                )
                summary["pulse_width"]["total_endpoints"] = int(
                    self._extract_numeric(values[11], 0)
                )

        return summary

    def _parse_clock_summary(self, clock_text: str) -> list:
        """Parse the clock summary section for clock definitions."""
        clocks = []

        # Find the clock table
        table_match = re.search(
            r"Clock\s+Waveform\(ns\).*?Period\(ns\).*?Frequency\(MHz\).*?\n\s*[-\s]+\n(.*?)(?=\n\s*\n|\Z)",
            clock_text,
            re.DOTALL,
        )

        if table_match:
            table_data = table_match.group(1)
            lines = [line.strip() for line in table_data.split("\n") if line.strip()]

            for line in lines:
                parts = line.split()
                if len(parts) >= 4:
                    clock_name = parts[0]
                    waveform_str = parts[1]
                    period = self._extract_numeric(parts[2], 0.0)
                    frequency = self._extract_numeric(parts[3], 0.0)

                    # Parse waveform
                    waveform = []
                    waveform_match = re.findall(r"[\d.]+", waveform_str)
                    for w in waveform_match:
                        waveform.append(float(w))

                    clock = {
                        "name": clock_name,
                        "period": period,
                        "waveform": waveform,
                        "frequency_mhz": frequency,
                    }
                    clocks.append(clock)

        return clocks

    def _parse_critical_paths(self, paths_text: str) -> list:
        """Parse the critical paths section for detailed path information."""
        paths = []
        path_id = 1

        # Split into individual path reports
        # Handle both section-based reports and direct slack reports
        if "Max Delay Paths" in paths_text or "Min Delay Paths" in paths_text:
            path_sections = re.split(r"(?=Slack\s*\([^)]+\)\s*:)", paths_text)
        else:
            # Direct timing report format
            path_sections = [paths_text]

        for section in path_sections:
            if not section.strip() or "Slack" not in section:
                continue

            path = self._parse_single_path(section, path_id)
            if path:
                paths.append(path)
                path_id += 1

        return paths

    def _parse_single_path(self, path_text: str, path_id: int) -> dict:
        """Parse a single critical path."""
        path = {"path_id": path_id}

        # Extract slack and status - handle both formats
        slack_match = re.search(r"Slack\s*\(([^)]+)\)\s*:\s*([-\d.]+)ns", path_text)
        if not slack_match:
            # Try alternative format "Slack: inf" or "Slack: X.XXX"
            slack_match = re.search(r"Slack\s*:\s*([-\d.]+|inf)", path_text)
            if slack_match:
                slack_value = slack_match.group(1)
                if slack_value == "inf":
                    path["slack"] = 999999.0  # Use large number instead of inf
                    path["status"] = "MET"
                else:
                    path["slack"] = self._extract_numeric(slack_value, 0.0)
                    path["status"] = "MET"  # Assume MET for positive slack
            else:
                path["status"] = "MET"
                path["slack"] = 0.0
        else:
            status = slack_match.group(1)
            slack = self._extract_numeric(slack_match.group(2), 0.0)
            path["status"] = "VIOLATED" if "VIOLATED" in status else "MET"
            path["slack"] = slack

        # Extract source and destination
        source_match = re.search(r"Source:\s*([^\n]+)", path_text)
        if source_match:
            source_info = source_match.group(1).strip()
            path["source"] = self._parse_endpoint(
                source_info, path_text, is_source=True
            )
        else:
            path["source"] = {
                "name": "unknown",
                "type": "input_port",
                "cell_type": "PORT",
                "clock": "clk",
            }

        dest_match = re.search(r"Destination:\s*([^\n]+)", path_text)
        if dest_match:
            dest_info = dest_match.group(1).strip()
            path["destination"] = self._parse_endpoint(
                dest_info, path_text, is_source=False
            )
        else:
            path["destination"] = {
                "name": "unknown",
                "type": "input_port",
                "cell_type": "PORT",
                "clock": "clk",
            }

        # Extract path group
        group_match = re.search(r"Path Group:\s*(\w+)", path_text)
        if group_match:
            path["path_group"] = group_match.group(1)
        else:
            path["path_group"] = "default"

        # Extract path type
        type_match = re.search(r"Path Type:\s*(\w+)", path_text)
        if type_match:
            path_type = type_match.group(1).lower()
            if path_type in ["setup", "hold", "pulse_width"]:
                path["path_type"] = path_type
            else:
                path["path_type"] = "setup"
        else:
            path["path_type"] = "setup"

        # Extract requirement
        req_match = re.search(r"Requirement:\s*([-\d.]+)ns", path_text)
        if req_match:
            path["timing_requirement"] = self._extract_numeric(req_match.group(1), 0.0)
        else:
            path["timing_requirement"] = 0.0

        # Extract data path delay and breakdown
        delay_match = re.search(
            r"Data Path Delay:\s*([-\d.]+)ns\s*\(logic\s*([-\d.]+)ns\s*"
            r"\(([-\d.]+)%\)\s*route\s*([-\d.]+)ns\s*\(([-\d.]+)%\)\)",
            path_text,
        )
        if delay_match:
            path["data_path_delay"] = self._extract_numeric(delay_match.group(1), 0.0)
            path["delay_breakdown"] = {
                "logic_delay": self._extract_numeric(delay_match.group(2), 0.0),
                "logic_delay_percentage": self._extract_numeric(
                    delay_match.group(3), 0.0
                ),
                "route_delay": self._extract_numeric(delay_match.group(4), 0.0),
                "route_delay_percentage": self._extract_numeric(
                    delay_match.group(5), 0.0
                ),
            }
        else:
            path["data_path_delay"] = 0.0
            path["delay_breakdown"] = {
                "logic_delay": 0.0,
                "logic_delay_percentage": 0.0,
                "route_delay": 0.0,
                "route_delay_percentage": 0.0,
            }

        # Extract logic levels
        levels_match = re.search(r"Logic Levels:\s*(\d+)\s*\(([^)]+)\)", path_text)
        if levels_match:
            total_levels = int(levels_match.group(1))
            breakdown_str = levels_match.group(2)
            breakdown = {}

            # Parse breakdown like "CARRY8=6 LUT2=1 LUT4=4"
            for item in breakdown_str.split():
                if "=" in item:
                    cell_type, count = item.split("=")
                    breakdown[cell_type] = int(count)

            path["logic_levels"] = {"total": total_levels, "breakdown": breakdown}
        else:
            path["logic_levels"] = {"total": 0, "breakdown": {}}

        # Extract other timing parameters
        input_delay_match = re.search(r"Input Delay:\s*([-\d.]+)ns", path_text)
        if input_delay_match:
            path["input_delay"] = self._extract_numeric(input_delay_match.group(1), 0.0)
        else:
            path["input_delay"] = 0.0

        skew_match = re.search(r"Clock Path Skew:\s*([-\d.]+)ns", path_text)
        if skew_match:
            path["clock_path_skew"] = self._extract_numeric(skew_match.group(1), 0.0)
        else:
            path["clock_path_skew"] = 0.0

        uncertainty_match = re.search(r"Clock Uncertainty:\s*([-\d.]+)ns", path_text)
        if uncertainty_match:
            path["clock_uncertainty"] = self._extract_numeric(
                uncertainty_match.group(1), 0.0
            )
        else:
            path["clock_uncertainty"] = 0.0

        # Parse data path and required time sections
        path["data_path"] = self._parse_path_timing_section(
            path_text, is_data_path=True
        )
        path["required_time_calculation"] = self._parse_path_timing_section(
            path_text, is_data_path=False
        )

        # Extract arrival and required times
        arrival_match = re.search(r"arrival time\s*([-\d.]+)", path_text)
        if arrival_match:
            path["arrival_time"] = self._extract_numeric(arrival_match.group(1), 0.0)
        else:
            path["arrival_time"] = 0.0

        required_match = re.search(r"required time\s*([-\d.]+)", path_text)
        if required_match:
            path["required_time"] = self._extract_numeric(required_match.group(1), 0.0)
        else:
            path["required_time"] = 0.0

        return path

    def _parse_endpoint(
        self, endpoint_info: str, full_path_text: str, is_source: bool
    ) -> dict:
        """Parse source or destination endpoint information."""
        endpoint = {}

        # Extract the main name (first line)
        endpoint["name"] = endpoint_info.split("\n")[0].strip()

        # Determine type based on context
        if "input port" in endpoint_info or "(IN)" in full_path_text:
            endpoint["type"] = "input_port"
        elif "output port" in endpoint_info:
            endpoint["type"] = "output_port"
        elif any(cell in endpoint_info for cell in ["FDRE", "FDCE", "FDPE"]):
            endpoint["type"] = "register"
        else:
            endpoint["type"] = "combinational"

        # Extract cell type for registers - set default if not found
        cell_match = re.search(r"(FDRE|FDCE|FDPE|LUT\d+|CARRY\d+)", endpoint_info)
        if cell_match:
            endpoint["cell_type"] = cell_match.group(1)
        else:
            # Set default cell type based on endpoint type for schema compliance
            if endpoint["type"] == "input_port":
                endpoint["cell_type"] = "PORT"
            elif endpoint["type"] == "output_port":
                endpoint["cell_type"] = "PORT"
            elif endpoint["type"] == "register":
                endpoint["cell_type"] = "FDRE"  # Default register type
            else:
                endpoint["cell_type"] = "COMB"  # Default for combinational

        # Extract clock information - ensure always present for schema compliance
        clock_match = re.search(r"clocked by (\w+)", endpoint_info)
        if clock_match:
            endpoint["clock"] = clock_match.group(1)
        elif "clk" in full_path_text:  # fallback
            endpoint["clock"] = "clk"
        else:
            # Set default clock for schema compliance
            endpoint["clock"] = "clk"

        return endpoint

    def _parse_path_timing_section(
        self, path_text: str, is_data_path: bool = True
    ) -> list:
        """Parse the detailed timing path section."""
        steps = []

        # Find the timing table
        table_match = re.search(
            r"Location\s+Delay type\s+Incr\(ns\)\s+Path\(ns\)\s+"
            r"Netlist Resource\(s\).*?\n\s*[-\s]+\n(.*?)(?=\n\s*[-\s]+\n|\Z)",
            path_text,
            re.DOTALL,
        )

        if table_match:
            table_data = table_match.group(1)
            lines = [line for line in table_data.split("\n") if line.strip()]

            for line in lines:
                if not line.strip() or line.strip().startswith("-"):
                    continue

                step = self._parse_timing_step_new(line, is_data_path)
                if step:
                    steps.append(step)

        return steps

    def _parse_timing_step_new(self, line: str, is_data_path: bool = True) -> dict:
        """Parse a single timing step from the path table."""
        # Split line into components, handling multi-space separation
        parts = re.split(r"\s{2,}", line.strip())
        if len(parts) < 3:
            return None

        step = {}

        # Location (first column, can be empty)
        location = parts[0].strip() if parts[0].strip() else None
        if location:
            step["location"] = location

        # Delay type (second column)
        delay_type = parts[1].strip()

        # Determine step type based on delay type and context
        if "clock" in delay_type.lower() and "rise edge" in delay_type.lower():
            step["step_type"] = "clock_edge"
            if "r" in delay_type:
                step["transition"] = "r"
            elif "f" in delay_type:
                step["transition"] = "f"
        elif "clock pessimism" in delay_type.lower():
            step["step_type"] = "clock_pessimism"
        elif "clock uncertainty" in delay_type.lower():
            step["step_type"] = "clock_uncertainty"
        elif "setup" in delay_type.lower():
            if is_data_path:
                step["step_type"] = "endpoint"
            else:
                step["step_type"] = "setup_requirement"
            detail_match = re.search(r"\(([^)]+)\)", delay_type)
            if detail_match:
                step["component_detail"] = detail_match.group(1)
        elif "input delay" in delay_type.lower():
            if is_data_path:
                step["step_type"] = "input_delay"
            else:
                step["step_type"] = "net"
        elif "net" in delay_type.lower():
            step["step_type"] = "net"
            # Extract transition from delay type like "net (fo=40, unset)"
            trans_match = re.search(r"\b([rf])\b", delay_type)
            if trans_match:
                step["transition"] = trans_match.group(1)
            # Extract fanout
            fo_match = re.search(r"fo=(\d+)", delay_type)
            if fo_match:
                step["fan_out"] = int(fo_match.group(1))
            else:
                step["fan_out"] = 1
        elif any(cell in delay_type for cell in ["LUT", "CARRY", "FDRE", "FDCE"]):
            if is_data_path:
                step["step_type"] = "cell"
            else:
                step["step_type"] = "endpoint"
            # Extract component detail from parentheses
            detail_match = re.search(r"\(([^)]+)\)", delay_type)
            if detail_match:
                step["component_detail"] = detail_match.group(1)
            # Extract component type
            comp_match = re.search(r"(LUT\d+|CARRY\d+|FDRE|FDCE|FDPE)", delay_type)
            if comp_match:
                step["component_type"] = comp_match.group(1)
        else:
            # Default to net if unclear
            step["step_type"] = "net"
            step["fan_out"] = 1

        # Incremental delay (third column)
        try:
            incr_delay = float(parts[2]) if parts[2] != "-" else 0.0
        except (ValueError, IndexError):
            incr_delay = 0.0
        step["incremental_delay"] = incr_delay

        # Path delay (fourth column)
        try:
            path_delay = float(parts[3]) if len(parts) > 3 and parts[3] != "-" else 0.0
        except (ValueError, IndexError):
            path_delay = 0.0

        if is_data_path:
            step["path_delay_before_routing"] = path_delay
            step["path_delay_after_routing"] = path_delay
        else:
            step["path_delay"] = path_delay

        # Resource (fifth column, if present)
        if len(parts) > 4:
            resource = parts[4].strip()
            if resource:
                step["resource"] = resource
                if step["step_type"] == "net":
                    step["net_name"] = resource

        # Assign logic_delay and routing_delay based on step type (data_path only)
        if is_data_path:
            if step["step_type"] == "cell":
                step["logic_delay"] = incr_delay
                step["routing_delay"] = 0.0
            elif step["step_type"] == "net":
                step["logic_delay"] = 0.0
                step["routing_delay"] = incr_delay
            else:
                step["logic_delay"] = 0.0
                step["routing_delay"] = 0.0

        # Extract transition from resource line if not already set
        if "transition" not in step and len(parts) > 4:
            if parts[4].strip().endswith(" r"):
                step["transition"] = "r"
            elif parts[4].strip().endswith(" f"):
                step["transition"] = "f"

        return step

    @staticmethod
    def _extract_numeric(text: str, default=None) -> float:
        """Extract and convert numeric value with robust error handling."""
        if text is None:
            return default

        # Remove common non-numeric characters and extract number
        cleaned = re.sub(r"[^\d.-]", "", str(text))
        if not cleaned:
            return default

        try:
            return float(cleaned)
        except ValueError:
            return default

    @staticmethod
    def _normalize_whitespace(text: str) -> str:
        """Normalize whitespace in text for consistent parsing."""
        return " ".join(text.split())
