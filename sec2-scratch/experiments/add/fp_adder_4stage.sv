// 4-stage pipelined floating point adder  
module FP_Adder_4Stage (
    input clk,
    input reset,
    input [31:0] a,
    input [31:0] b,
    output reg [31:0] result
);

// Stage 1 registers
reg [7:0] e1_s1, e2_s1, larger_exp_s1;
reg [22:0] m1_s1, m2_s1, small_mantissa_s1, large_mantissa_s1;
reg s1_s1, s2_s1;

// Stage 2 registers
reg [23:0] add_mantissa_s2;
reg [7:0] larger_exp_s2, e1_s2, e2_s2;
reg [22:0] m1_s2, m2_s2;
reg s1_s2, s2_s2;

// Stage 3 registers
reg [23:0] norm_mantissa_s3;
reg [7:0] final_exp_s3;
reg final_sign_s3;

// Stage 4 registers
reg [31:0] result_s4;

always @(posedge clk) begin
    if (reset) begin
        // Reset all stages
        e1_s1 <= 0; e2_s1 <= 0; larger_exp_s1 <= 0;
        m1_s1 <= 0; m2_s1 <= 0; small_mantissa_s1 <= 0; large_mantissa_s1 <= 0;
        s1_s1 <= 0; s2_s1 <= 0;
        
        add_mantissa_s2 <= 0; larger_exp_s2 <= 0; e1_s2 <= 0; e2_s2 <= 0;
        m1_s2 <= 0; m2_s2 <= 0; s1_s2 <= 0; s2_s2 <= 0;
        
        norm_mantissa_s3 <= 0; final_exp_s3 <= 0; final_sign_s3 <= 0;
        
        result_s4 <= 0;
        result <= 0;
    end else begin
        // Stage 1: Input processing and mantissa alignment
        s1_s1 <= a[31]; s2_s1 <= b[31];
        e1_s1 <= a[30:23]; e2_s1 <= b[30:23];
        m1_s1 <= a[22:0]; m2_s1 <= b[22:0];
        
        if (a[30:23] > b[30:23]) begin
            larger_exp_s1 <= a[30:23];
            large_mantissa_s1 <= (a[30:23] != 0) ? {1'b1, a[22:1]} : a[22:0];
            small_mantissa_s1 <= ((b[30:23] != 0) ? {1'b1, b[22:1]} : b[22:0]) >> (a[30:23] - b[30:23]);
        end else begin
            larger_exp_s1 <= b[30:23];
            large_mantissa_s1 <= (b[30:23] != 0) ? {1'b1, b[22:1]} : b[22:0];
            small_mantissa_s1 <= ((a[30:23] != 0) ? {1'b1, a[22:1]} : a[22:0]) >> (b[30:23] - a[30:23]);
        end
        
        // Stage 2: Mantissa addition/subtraction
        s1_s2 <= s1_s1; s2_s2 <= s2_s1;
        e1_s2 <= e1_s1; e2_s2 <= e2_s1; m1_s2 <= m1_s1; m2_s2 <= m2_s1;
        larger_exp_s2 <= larger_exp_s1;
        
        if (s1_s1 == s2_s1) begin
            add_mantissa_s2 <= large_mantissa_s1 + small_mantissa_s1;
        end else begin
            add_mantissa_s2 <= large_mantissa_s1 - small_mantissa_s1;
        end
        
        // Stage 3: Normalization
        if (add_mantissa_s2[23]) begin
            norm_mantissa_s3 <= add_mantissa_s2 >> 1;
            final_exp_s3 <= larger_exp_s2 + 1;
        end else if (add_mantissa_s2[22]) begin
            norm_mantissa_s3 <= add_mantissa_s2;
            final_exp_s3 <= larger_exp_s2;
        end else begin
            // Find first 1 bit and normalize
            reg [4:0] shift_amt;
            shift_amt = 0;
            if (add_mantissa_s2 != 0) begin
                while (!add_mantissa_s2[22 - shift_amt] && shift_amt < 23) 
                    shift_amt = shift_amt + 1;
                norm_mantissa_s3 <= add_mantissa_s2 << shift_amt;
                final_exp_s3 <= larger_exp_s2 - shift_amt;
            end else begin
                norm_mantissa_s3 <= 0;
                final_exp_s3 <= 0;
            end
        end
        
        // Determine sign
        if (s1_s2 == s2_s2) begin
            final_sign_s3 <= s1_s2;
        end else if (e1_s2 > e2_s2) begin
            final_sign_s3 <= s1_s2;
        end else if (e2_s2 > e1_s2) begin
            final_sign_s3 <= s2_s2;
        end else if (m1_s2 > m2_s2) begin
            final_sign_s3 <= s1_s2;
        end else begin
            final_sign_s3 <= s2_s2;
        end
        
        // Stage 4: Result formation
        result_s4 <= {final_sign_s3, final_exp_s3, norm_mantissa_s3[22:0]};
        
        // Output
        result <= result_s4;
    end
end

endmodule
