module Init (
  input logic _go,
  output logic [8:0] acc,
  input logic [7:0] left,
  output logic [7:0] quotient
);
  assign {acc, quotient} = {{8{1'b0}}, left, 1'b0};
endmodule

module Next(
  input logic _go,
  input logic [8:0] acc,
  input logic [7:0] right,
  input logic [7:0] quotient,
  output logic [8:0] acc_next,
  output logic [7:0] quotient_next
);
    always_comb begin
      if (acc >= {1'b0, right}) begin
        acc_next = acc - right;
        {acc_next, quotient_next} = {acc_next[8-1:0], quotient, 1'b1};
      end else begin
        {acc_next, quotient_next} = {acc, quotient} << 1;
      end
    end
endmodule
