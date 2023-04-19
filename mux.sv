// Either a full expression (useful for multiple triggers)
(* trigger="posedge i_clk" *)
module mux(
    // (* trigger *) // Or you can specify a port and it will use @(posedge <the port>)
    input var logic i_clk,
    input var logic i_rst,
    input var logic i_sel,
    input var logic i_a,
    input var logic i_b,
    input var logic[120:0] i_double,
    input var logic[7:0][15:0] i_u16_array,
    output var logic o_aorb,
    output var logic[3:0] o_slice,
    output var logic[64:0] o_double_slice,
    // If you need more than 128 bits you can do an array like this!
    output var logic[1:0][127:0] o_wide
);

endmodule;
