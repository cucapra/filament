
module __SchedulingDelayShift #(
  parameter WIDTH = 32,
  parameter DELAY = 0,
  parameter LIVE = 1
) (
  input wire clk,
  input wire reset,
  input wire logic [WIDTH-1:0] in, // ['G, 'G+1]
  output logic [WIDTH-1:0] out // ['G+DELAY, 'G+DELAY+1]
);

  // Basic shift register
  logic [WIDTH-1:0] shift[DELAY-1:0];
  always @(posedge clk) begin
    if (reset) begin
      shift[0] <= 0;
    end else begin
      shift[0] <= in;
    end
  end
  for (genvar i = 1; i < DELAY; i++) begin
    always @(posedge clk) begin
      if (reset) begin
        shift[i] <= 0;
      end else begin
        shift[i] <= shift[i-1];
      end
    end
  end

  assign out = shift[DELAY-1];
endmodule


module __SchedulingDelayRegister #(
  parameter WIDTH = 32,
  parameter DELAY = 0,
  parameter LIVE = 1
) (
  input wire clk,
  input wire reset,
  input wire logic write_en,
  input wire logic [WIDTH-1:0] in, // ['G, 'G+1]
  output logic [WIDTH-1:0] out // ['G+DELAY, 'G+DELAY+LIVE]
);
  generate
    case (DELAY)
      0: begin
        // This is a pass-through register
        logic [WIDTH-1:0] held;
        always @(posedge clk) begin
          if (reset)
            held <= 0;
          else if (write_en)
            held <= in;
          else
            held <= held;
        end

        assign out = (write_en ? in : held);
      end
      1: begin
        // This is a normal register
        always @(posedge clk) begin
          if (reset)
            out <= 0;
          else if (write_en)
            out <= in;
          else
            out <= out;
        end
      end
      2: begin
        logic fsm;
        logic [WIDTH-1:0] held;

        always @(posedge clk) begin
          if (reset) begin
            fsm <= 0;
            held <= 0;
          end
          else begin
            fsm <= write_en;
            held <= in;
          end
        end

        always @(posedge clk) begin
          if (reset)
            out <= 0;
          else if (fsm)
            out <= held;
          else
            out <= out;
        end
      end
      default: begin
        // valid from ['G+i+1, 'G+i+2]
        logic [DELAY-1:1] fsm;
        logic [DELAY-1:1][WIDTH-1:0] shift;

        always @(posedge clk) begin
          if (reset) begin
            fsm <= 0;
            shift[1] <= 0;
          end
          else begin
            fsm[1] <= write_en;
            fsm[DELAY-1:2] <= fsm[DELAY-2:1];
            shift[1] <= in;
          end
        end

        for (genvar i = 2; i < DELAY; i++) begin
          always @(posedge clk) begin
            if (reset) begin
              shift[i] <= 0;
            end else begin
              shift[i] <= shift[i-1];
            end
          end
        end

        // Add a register at the end
        always @(posedge clk) begin
          if (reset)
            out <= 0;
          else if (fsm[DELAY-1])
            out <= shift[DELAY-1];
          else
            out <= out;
        end
      end
    endcase
  endgenerate
endmodule