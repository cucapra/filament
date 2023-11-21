
// Unsafely takes a port of availability ['G, 'G+1] and ['G+1, 'G+2] and merges them into ['G, 'G+2]
module UnsafeMerge #(
    parameter W = 32
) (
    input logic go,
    input logic [W-1:0] in0,
    input logic [W-1:0] in1,
    output logic [W-1:0] out
);
  assign out = go ? in0 : in1;
endmodule
