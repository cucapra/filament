// 1-stage pipelined floating point multiplier
module FP_Mult_1Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result,
    output reg exception,
    output reg overflow,
    output reg underflow
);

reg [31:0] a_reg, b_reg;

always @(posedge clk) begin
    if (reset) begin
        a_reg <= 0;
        b_reg <= 0;
        result <= 0;
        exception <= 0;
        overflow <= 0;
        underflow <= 0;
    end else begin
        a_reg <= a;
        b_reg <= b;
        
        // Compute multiplication result
        {result, exception, overflow, underflow} <= compute_mult(a_reg, b_reg);
    end
end

function automatic [34:0] compute_mult;  // {result[31:0], exception, overflow, underflow}
    input [31:0] num_a, num_b;
    
    reg sign, round, normalised, zero;
    reg [8:0] exponent, sum_exponent, exp_norm;
    reg [22:0] product_mantissa;
    reg [23:0] op_a, op_b;
    reg [47:0] product, product_normalised;
    reg exc, ovf, unf;
    reg [31:0] res;
    
    begin
        sign = num_a[31] ^ num_b[31];
        exc = (&num_a[30:23]) | (&num_b[30:23]);
        
        // Extract operands with hidden bit
        op_a = (|num_a[30:23]) ? {1'b1, num_a[22:0]} : {1'b0, num_a[22:0]};
        op_b = (|num_b[30:23]) ? {1'b1, num_b[22:0]} : {1'b0, num_b[22:0]};
        
        product = op_a * op_b;
        normalised = product[47] ? 1'b1 : 1'b0;
        product_normalised = normalised ? product : product << 1;
        
        round = |product_normalised[22:0];
        product_mantissa = product_normalised[46:24] + (product_normalised[23] & round);
        
        sum_exponent = num_a[30:23] + num_b[30:23];
        exp_norm = sum_exponent - 8'd127;
        exponent = exp_norm + normalised;
        
        ovf = (exponent[8] & !exc & !exponent[7]);
        unf = (exponent[8] & !exc & exponent[7]);
        
        res = exc ? {sign, 31'd0} :
              ovf ? {sign, 8'hFF, 23'd0} :
              unf ? {sign, 31'd0} :
              {sign, exponent[7:0], product_mantissa};
              
        compute_mult = {res, exc, ovf, unf};
    end
endfunction

endmodule
