module BlackBoxMulUInt8(
	               input wire [7:0] I0,
	               input wire [7:0] I1,
	               output wire [15:0] O,
	               input wire       clk
	               );
`ifdef ICARUS
   reg [15:0] reg0;
   reg [15:0] reg1;
   reg [15:0] reg2;
   always @( posedge clk ) begin
      reg0 <= I0 * I1;
      reg1 <= reg0;
      reg2 <= reg1;
   end
   assign O = reg2;
`endif
`ifndef ICARUS
   mul_uint8 mul_uint8 (
                   .A(I0),
                   .B(I1),
                   .P(O),
                   .CLK(clk)
                   );
`endif
endmodule