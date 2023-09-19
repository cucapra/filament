/// Reference implementation for an iterative division algorithm.
module IterDiv #(
  parameter W = 32
) (
    input  logic             go,
    input  logic             clk,
    input  logic             reset,
    input  logic [W-1:0] left,
    input  logic [W-1:0] right,
    output logic [W-1:0] out_remainder,
    output logic [W-1:0] out_quotient,
    output logic             done
);
    localparam ITERATIONS = W;

    logic [W-1:0] quotient, quotient_next;
    logic [W:0] acc, acc_next;
    logic [$clog2(ITERATIONS):0] idx;
    logic start, running, finished, dividend_is_zero;

    assign start = go && !running;
    // NOTE: 0 / 0 returns 0 due to this optimization.
    // Otherwise, would return (1 << W) - 1
    assign dividend_is_zero = start && left == 0;
    assign finished = idx == ITERATIONS - 1 && running;

    always_ff @(posedge clk) begin
      if (reset || finished || dividend_is_zero)
        running <= 0;
      else if (start)
        running <= 1;
      else
        running <= running;
    end

    always_comb begin
      if (acc >= {1'b0, right}) begin
        acc_next = acc - right;
        {acc_next, quotient_next} = {acc_next[W-1:0], quotient, 1'b1};
      end else begin
        {acc_next, quotient_next} = {acc, quotient} << 1;
      end
    end

    // `done` signaling
    always_ff @(posedge clk) begin
      if (dividend_is_zero || finished)
        done <= 1;
      else
        done <= 0;
    end

    always_ff @(posedge clk) begin
      if (running)
        idx <= idx + 1;
      else
        idx <= 0;
    end

    always_ff @(posedge clk) begin
      if (reset) begin
        out_quotient <= 0;
        out_remainder <= 0;
      end else if (start) begin
        out_quotient <= 0;
        out_remainder <= left;
      end else if (dividend_is_zero) begin
        out_quotient <= 0;
        out_remainder <= 0;
      end else if (finished) begin
        out_quotient <= quotient_next;
        out_remainder <= out_remainder;
      end else begin
        out_quotient <= out_quotient;
        if (out_remainder >= right)
          out_remainder <= out_remainder - right;
        else
          out_remainder <= out_remainder;
      end
    end

    always_ff @(posedge clk) begin
      if (reset) begin
        acc <= 0;
        quotient <= 0;
      end else if (start) begin
        {acc, quotient} <= {{W{1'b0}}, left, 1'b0};
      end else begin
        acc <= acc_next;
        quotient <= quotient_next;
      end
    end
endmodule