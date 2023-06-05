module regfile (
  input wire clk,
  input wire reset,
  input wire logic en,
  input wire logic [4:0] rin1,
  input wire logic [4:0] rin2,
  input wire logic [4:0] rd,
  input wire logic [31:0] data,
  output reg [31:0] rs1,
  output reg [31:0] rs2
);
  integer i;
  reg [31:0] rf [31:0];
  assign rs1 = rf[rin1];
  assign rs2 = rf[rin2];

  always_ff @(posedge clk) begin 
    if (reset) begin 
      for (i = 0; i < 32; i = i+1) begin rf[i] <= 32'd0; end
    end else if (en) begin
      rf[rd] <= data;
    end
  end
endmodule