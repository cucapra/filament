// 2-stage pipelined floating point multiplier
module FP_Mult_2Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result,
    output reg exception,
    output reg overflow,
    output reg underflow
);

// Stage 1 registers
reg sign_s1, exc_s1;
reg [23:0] op_a_s1, op_b_s1;
reg [8:0] sum_exp_s1;

// Stage 2 registers
reg sign_s2, exc_s2;
reg [47:0] product_s2;
reg [8:0] sum_exp_s2;

always @(posedge clk) begin
    if (reset) begin
        // Reset stage 1
        sign_s1 <= 0; exc_s1 <= 0;
        op_a_s1 <= 0; op_b_s1 <= 0;
        sum_exp_s1 <= 0;
        
        // Reset stage 2
        sign_s2 <= 0; exc_s2 <= 0;
        product_s2 <= 0; sum_exp_s2 <= 0;
        
        result <= 0;
        exception <= 0;
        overflow <= 0;
        underflow <= 0;
    end else begin
        // Stage 1: Input processing and setup
        sign_s1 <= a[31] ^ b[31];
        exc_s1 <= (&a[30:23]) | (&b[30:23]);
        op_a_s1 <= (|a[30:23]) ? {1'b1, a[22:0]} : {1'b0, a[22:0]};
        op_b_s1 <= (|b[30:23]) ? {1'b1, b[22:0]} : {1'b0, b[22:0]};
        sum_exp_s1 <= a[30:23] + b[30:23];
        
        // Stage 2: Multiplication and result formation
        sign_s2 <= sign_s1;
        exc_s2 <= exc_s1;
        product_s2 <= op_a_s1 * op_b_s1;
        sum_exp_s2 <= sum_exp_s1;
        
        // Generate final result
        {result, exception, overflow, underflow} <= form_result(sign_s2, exc_s2, product_s2, sum_exp_s2);
    end
end

function [34:0] form_result;  // {result[31:0], exception, overflow, underflow}
    input sign, exc;
    input [47:0] product;
    input [8:0] sum_exp;
    
    reg normalised, round;
    reg [8:0] exponent, exp_norm;
    reg [22:0] product_mantissa;
    reg [47:0] product_normalised;
    reg ovf, unf;
    reg [31:0] res;
    
    begin
        normalised = product[47] ? 1'b1 : 1'b0;
        product_normalised = normalised ? product : product << 1;
        
        round = |product_normalised[22:0];
        product_mantissa = product_normalised[46:24] + (product_normalised[23] & round);
        
        exp_norm = sum_exp - 8'd127;
        exponent = exp_norm + normalised;
        
        ovf = (exponent[8] & !exc & !exponent[7]);
        unf = (exponent[8] & !exc & exponent[7]);
        
        res = exc ? {sign, 31'd0} :
              ovf ? {sign, 8'hFF, 23'd0} :
              unf ? {sign, 31'd0} :
              {sign, exponent[7:0], product_mantissa};
              
        form_result = {res, exc, ovf, unf};
    end
endfunction

endmodule