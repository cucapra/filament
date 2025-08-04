import re
import logging as log

"""
Parses timing reports that look like this:

```
Max Delay Paths
--------------------------------------------------------------------------------------
Slack (VIOLATED) :        -0.317ns  (required time - arrival time)
  Source:                 a[23]
                            (input port clocked by clk  {rise@0.000ns fall@3.500ns period=7.000ns})
  Destination:            alu/adder/pipeline/single_stage.data_out_reg[29]/D
                            (rising edge-triggered cell FDRE clocked by clk  {rise@0.000ns fall@3.500ns period=7.000ns})
  Path Group:             clk
  Path Type:              Setup (Max at Slow Process Corner)
  Requirement:            7.000ns  (clk rise@7.000ns - clk rise@0.000ns)
  Data Path Delay:        8.214ns  (logic 1.948ns (23.716%)  route 6.266ns (76.285%))
  Logic Levels:           18  (CARRY8=5 LUT3=1 LUT5=3 LUT6=9)
  Input Delay:            0.500ns
  Clock Path Skew:        1.405ns (DCD - SCD + CPR)
    Destination Clock Delay (DCD):    1.405ns = ( 8.405 - 7.000 )
    Source Clock Delay      (SCD):    0.000ns
    Clock Pessimism Removal (CPR):    0.000ns
  Clock Uncertainty:      0.035ns  ((TSJ^2 + TIJ^2)^1/2 + DJ) / 2 + PE
    Total System Jitter     (TSJ):    0.071ns
    Total Input Jitter      (TIJ):    0.000ns
    Discrete Jitter          (DJ):    0.000ns
    Phase Error              (PE):    0.000ns

    Location             Delay type                Incr(ns)  Path(ns)    Netlist Resource(s)
  -------------------------------------------------------------------    -------------------
                         (clock clk rise edge)        0.000     0.000 r
                         input delay                  0.500     0.500
                                                      0.000     0.500 r  a[23] (IN)
                         net (fo=40, unset)           0.000     0.500    alu/adder/a[23]
    SLICE_X22Y14         LUT6 (Prop_F6LUT_SLICEL_I0_O)
                                                      0.177     0.677 r  alu/adder/i__carry_i_136/O
                         net (fo=3, routed)           0.270     0.947    alu/adder/i__carry_i_136_n_0
    SLICE_X22Y14         LUT5 (Prop_D6LUT_SLICEL_I0_O)
                                                      0.174     1.121 r  alu/adder/i__carry_i_183/O
                         net (fo=1, routed)           0.104     1.225    alu/adder/i__carry_i_183_n_0
    SLICE_X22Y14         LUT6 (Prop_G6LUT_SLICEL_I3_O)
                                                      0.114     1.339 f  alu/adder/i__carry_i_135/O
                         net (fo=39, routed)          0.504     1.843    alu/adder/i__carry_i_135_n_0
    SLICE_X24Y6          LUT6 (Prop_C6LUT_SLICEM_I5_O)
                                                      0.098     8.637 r  alu/adder/pipeline/single_stage.data_out[29]_i_1/O
                         net (fo=1, routed)           0.077     8.714    alu/adder/pipeline/sum_next[29]
    SLICE_X24Y6          FDRE                                         r  alu/adder/pipeline/single_stage.data_out_reg[29]/D
  -------------------------------------------------------------------    -------------------

                         (clock clk rise edge)        7.000     7.000 r
                                                      0.000     7.000 r  clk (IN)
                         net (fo=64, unset)           1.405     8.405    alu/adder/pipeline/clk
    SLICE_X24Y6          FDRE                                         r  alu/adder/pipeline/single_stage.data_out_reg[29]/C
                         clock pessimism              0.000     8.405
                         clock uncertainty           -0.035     8.370
    SLICE_X24Y6          FDRE (Setup_CFF_SLICEM_C_D)
                                                      0.027     8.397    alu/adder/pipeline/single_stage.data_out_reg[29]
  -------------------------------------------------------------------
                         required time                          8.397
                         arrival time                          -8.714
  -------------------------------------------------------------------
                         slack                                 -0.317
```
"""


class TimingParser:
    def __init__(self, contents):
        # We store the contents of the file as an array of lines
        self.contents = contents
        self.data = {}

    def parse_logic_levels(self, req: str) -> str:
        """
        Parses the requirement line, which contains the required time.
        ```
        18  (CARRY8=5 LUT3=1 LUT5=3 LUT6=9)
        ```
        """
        data = {}
        match = re.match(r"(\d+)\s+\((.*)\)", req)
        if match:
            data["levels"] = int(match.group(1))
            for detail in match.group(2).split():
                if "=" in detail:
                    key, value = detail.split("=")
                    data[key] = int(value)
                else:
                    raise ValueError(f"Invalid logic level detail: {detail}")
        else:
            raise ValueError(f"Invalid logic levels format: {req}")

        return data

    def parse_header_data_path_delay(self, req: str) -> dict:
        """
        Parses the data path delay line, which contains the delay information.
        ```
        8.214ns  (logic 1.948ns (23.716%)  route 6.266ns (76.285%))
        ```
        """
        data = {}
        match = re.match(
            r"([\d.]+)ns\s*\(logic\s*([\d.]+)ns\s*\(([\d.]+)%\)\s*route\s*([\d.]+)ns\s*\(([\d.]+)%\)\)",
            req,
        )
        assert match, f"Invalid data path delay format: {req}"
        data["total"] = float(match.group(1))
        data["logic"] = float(match.group(2))
        data["logic_percent"] = float(match.group(3))
        data["route"] = float(match.group(4))
        data["route_percent"] = float(match.group(5))
        return data

    def parse_slack(self, req: str) -> dict:
        """
        Parses the slack line, which contains slack status and value
        ```
        Slack (VIOLATED) :        -0.317ns  (required time - arrival time)
        ```
        """
        data = {}
        match = re.match(r"Slack\s*\((\w+)\)\s*:\s*([\d.-]+)ns", req)
        assert match, f"Invalid slack format: {req}"
        data["status"] = match.group(1)
        data["slack"] = float(match.group(2))
        return data

    def parse_critical_path_header(self, start: int, end: int):
        """
        Parses the header of a critical path. The first line must start with "Slack" and contain the slack value.
        """
        data = {}
        for line in self.contents[start:end]:
            line = line.strip()
            # Each header line looks like `word: information`. If it doesn't match this format, skip it.
            match = re.match(r"\s*(.*):\s*(.*)", line)
            if not match:
                continue

            key, value = match.groups()
            if "Slack" in key:
                slack_data = self.parse_slack(line)
                data.update(slack_data)
            elif key in [
                "Source",
                "Destination",
            ]:
                data[key.lower().replace(" ", "_")] = value
            if key in ["Requirement", "Input Delay"]:
                # Requirement is in the format "7.000ns  (clk rise@7.000ns - clk rise@0.000ns)"
                match = re.match(r"([\d.]+)ns", value)
                assert match, f"Invalid requirement format: {value}"
                data[key.lower().replace(" ", "_")] = float(match.group(1))
            elif key == "Data Path Delay":
                data["data_path_delay"] = self.parse_header_data_path_delay(value)
            elif key == "Logic Levels":
                data["logic_levels"] = self.parse_logic_levels(value)

        return data

    def parse_loc(self, resource: str) -> dict:
        """
        Parses the location of a resource in the format "SLICE_X22Y14" or "SLICE_X24Y6".
        """
        # Trim whitespace
        resource = resource.strip()
        # Match with regex ((\w+)_X(\d+)Y(\d+))
        # group 1 is the type (e.g., SLICE), group 2 is X coordinate, and group 3 is Y coordinate.
        match = re.match(r"(\w+)_X(\d+)Y(\d+)", resource)
        if match:
            return {
                # "type": match.group(1),  # XXX: Discard the type information since it doesn't seem useful
                "x": int(match.group(2)),
                "y": int(match.group(3)),
            }
        else:
            raise ValueError(f"Invalid resource format: {resource}")

    def parse_critical_path_entry(self, idx: int) -> dict:
        """
        Parses one timing entry in the table. Each timing entry has three
        lines:
        ```
        SLICE_X22Y14    LUT6 (Prop_F6LUT_SLICEL_I0_O)
                                                     0.177     0.677 r  alu/adder/i__carry_i_136/O
                        net (fo=3, routed)           0.270     0.947    alu/adder/i__carry_i_136_n_0
        ```
        """
        data = {}
        log.info(f"Parsing entry: {idx} - {idx+3}")

        # Split the header using spaces, remove the empty elements.
        # Expect there are exactly 3 parts: location, resource, and delay type.
        # If not, raise an error.
        header = self.contents[idx].strip()
        parts = list(filter(None, header.split(" ")))
        assert len(parts) == 3, f"Invalid header start: {header}"

        loc, resource_kind, delay_type = parts
        data["loc"] = self.parse_loc(loc)
        data["kind"] = resource_kind
        data["delay_type"] = delay_type

        # The delay information line should have four elements: increment,
        # path, edge, resource.
        delay_info = self.contents[idx + 1].strip()
        delay_parts = list(filter(None, delay_info.split(" ")))
        if len(delay_parts) != 4:
            raise ValueError(f"Invalid delay info format: {delay_info}")
        data["logic_delay"] = float(delay_parts[0])
        data["edge"] = delay_parts[2]
        data["resource_name"] = delay_parts[3]

        # The net information line should has several elements:
        # the fanout, status, increment, path delay, and resource.
        net_info = self.contents[idx + 2].strip()

        # Split the list on ")" which is the end of the net information.
        net_info = net_info.split(")")
        assert len(net_info) == 2, f"Invalid net info format: {net_info}"

        # The first part contains the fanout information, which is in the
        # format "net (fo=40, routed)".
        fanout_info = net_info[0].strip()
        fanout_match = re.match(r"(\w+)\s*\(fo=(\d+)", fanout_info)
        assert fanout_match, f"Invalid fanout format: {fanout_info}"
        assert (
            fanout_match.group(1) == "net"
        ), f"Expected 'net' but got {fanout_match.group(1)}"
        data["fan_out"] = int(fanout_match.group(2))

        # The second part contains the delay and net information.
        remaining_info = net_info[1].strip()
        net_parts = list(filter(None, remaining_info.split(" ")))

        assert len(net_parts) == 3, f"Invalid net parts format: {net_parts}"
        data["net_delay"] = float(net_parts[0])
        data["net_name"] = net_parts[2]

        # Return the parsed data
        return data

    def parse_path(self, skip_first: bool, start: int, end: int) -> list:
        """
        Parses the critical path from start to the end index.
        If `skip_first` is set, we need to skip the first four entries because
        this file uses FORMAT 2.
        """
        idx = start + 4 if skip_first else start
        out = []
        while idx + 3 < end:
            entry = self.parse_critical_path_entry(idx)
            out.append(entry)
            idx += 3  # Move to the next entry

        return out

    def separator_line(self, line: str) -> bool:
        """
        Checks if the line is a separator line, which only contains dashes.
        """
        splits = list(filter(None, line.split(" ")))
        return len(splits) > 0 and all(
            [all(c == "-" for c in chunk) for chunk in splits]
        )

    def header_line(self, idx: int, title: str) -> bool:
        """
        A header line has a separator line after it and the separator line
        does not start with a space.
        """
        if idx + 1 >= len(self.contents):
            return False

        line = self.contents[idx].strip()
        next_line = self.contents[idx + 1]
        out = (
            line == title
            and not next_line.startswith(" ")
            and self.separator_line(next_line.strip())
        )
        return out

    def find_table(self, title: str) -> (int, int):
        """
        Find a timing report table with the give title.
        The end is detected when we find three consecutive empty lines.
        """

        def three_empty_lines(start: int) -> bool:
            """Check if there are three consecutive empty lines starting from
            index."""
            return (
                start + 2 < len(self.contents)
                and self.contents[start].strip() == ""
                and self.contents[start + 1].strip() == ""
                and self.contents[start + 2].strip() == ""
            )

        start = None
        end = None

        # Find the start of the table by looking for the title.
        # The title is expected to be a header line.
        for i, line in enumerate(self.contents):
            if self.header_line(i, title):
                start = i
                break
        else:
            raise ValueError(f"Table '{title}' not found in the report.")

        # Find the end of the table by looking for three consecutive empty lines.
        for i in range(start + 1, len(self.contents)):
            if three_empty_lines(i):
                end = i
                break

        if end is None:
            # If we didn't find three empty lines, assume the end is the end of
            # the file.
            end = len(self.contents)

        return start, end

    def parse_critical_path(self, start: int, end: int) -> dict:
        """
        Parses a critical path from the report.
        There are two possible formats:

        FORMAT 1:
        ```
        META
        -------
        CLOCK
        -------
        PATH
        -------
        EST
        -------
        COMPUTE
        -------
        SLACK
        ```

        FORMAT 2:
        ```
        META
        -------
        PATH
        -------
        EST
        -------
        COMPUTE
        -------
        SLACK
        ```
        """
        data = {}
        clock_start = None
        path_start = None
        path_end = None
        est_end = None
        compute_end = None
        log.info(f"CP Entry: {start} - {end}")

        # Walk over the lines and every time we find a separator line, we
        # assume that the previous path has ended.
        for i in range(start, end):
            line = self.contents[i].strip()
            if self.separator_line(line):
                if clock_start is None:
                    clock_start = i + 1
                    continue
                elif path_start is None:
                    path_start = i + 1
                    continue
                elif path_end is None:
                    path_end = i
                    continue
                elif est_end is None:
                    est_end = i
                    continue
                elif compute_end is None:
                    compute_end = i
                    continue
                else:
                    # path = "".join(self.contents[start:end])
                    path = ""
                    assert (
                        False
                    ), f"\n{path}\nUnexpected separator line found at line {i}!"

        log.info(
            f"Clock start: {clock_start}, Path start: {path_start}, "
            f"Path end: {path_end}, "
            f"EST end: {est_end}, Compute end: {compute_end}"
        )
        assert clock_start is not None, "No clock start found!"
        assert path_start is not None, "No path start found!"
        assert path_end is not None, "No path end found!"
        assert est_end is not None, "No estimated end found!"

        header_end = None
        skip_first = None
        if compute_end is None:
            # This is format 2. We need to shift things around
            est_end = path_end
            path_end = path_start
            path_start = clock_start
            header_end = path_start
            skip_first = True
        else:
            header_end = clock_start
            skip_first = False

        data["header"] = self.parse_critical_path_header(start, header_end)
        data["critical_path"] = self.parse_path(skip_first, path_start, path_end)
        return data

    def parse(self) -> dict:
        """
        Parses the entire timing report and returns a dictionary with the parsed
        data.
        """

        # Find the line with "Max Delay Paths" and start parsing from there.
        (t_start, t_end) = self.find_table("Max Delay Paths")
        log.info(f"Table bounds: {t_start} - {t_end}")

        # Find all indices that have work "Slack" in them
        slack_indices = [
            t_start + i
            for i, line in enumerate(self.contents[t_start:t_end])
            if "Slack" in line
        ]

        # For each line, find the end by using the next "Slack" and walking
        # backward till the first non-empty line is found.
        paths = []
        for i in range(len(slack_indices)):
            start = slack_indices[i]
            end = slack_indices[i + 1] if i + 1 < len(slack_indices) else t_end

            # Find the first non-empty line before the end
            while end > start and not self.contents[end - 1].strip():
                end -= 1

            # print(self.contents[start:end])
            # Parse the path from start to end
            paths.append(self.parse_critical_path(start, end))

        return paths


def log_setup():
    # Color labels for logging
    log.addLevelName(log.DEBUG, "\033[94mDEBUG\033[0m")
    log.addLevelName(log.INFO, "\033[92mINFO\033[0m")
    log.addLevelName(log.WARNING, "\033[93mWARNING\033[0m")
    log.addLevelName(log.ERROR, "\033[91mERROR\033[0m")
    log.addLevelName(log.CRITICAL, "\033[95mCRITICAL\033[0m")

    # Format log messages using [LOG_LEVEL] message
    log.basicConfig(
        format="[%(levelname)s] %(message)s",
        level=log.INFO,  # Default to INFO level
    )


if __name__ == "__main__":
    # Take the path information using argparse, parse the file and store its
    # contents, parse the output, and dump the JSON
    import argparse
    import json

    parser = argparse.ArgumentParser(description="Parse timing reports.")
    parser.add_argument("file", type=str, help="Path to the timing report file")
    parser.add_argument(
        "--log-level", type=str, default="WARN", help="Set the logging level"
    )
    args = parser.parse_args()

    # Set the logging level based on the command line argument
    log_setup()
    log.basicConfig(level=getattr(log, args.log_level.upper(), log.WARN))

    with open(args.file, "r") as f:
        contents = f.readlines()

    parser = TimingParser(contents)
    data = parser.parse()

    print(json.dumps(data, indent=2))
