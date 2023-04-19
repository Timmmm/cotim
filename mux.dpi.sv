
    import "DPI-C" function chandle mux_new();
    import "DPI-C" function void mux_free(input chandle ___instance);
    import "DPI-C" function byte unsigned mux_tick(input chandle ___instance, input bit[0:0] i_clk, input bit[0:0] i_rst, input bit[0:0] i_sel, input bit[0:0] i_a, input bit[0:0] i_b, input bit[120:0] i_double, input bit[15:0] i_u16_array___0, input bit[15:0] i_u16_array___1, input bit[15:0] i_u16_array___2, input bit[15:0] i_u16_array___3, input bit[15:0] i_u16_array___4, input bit[15:0] i_u16_array___5, input bit[15:0] i_u16_array___6, input bit[15:0] i_u16_array___7, output bit[0:0] o_aorb, output bit[3:0] o_slice, output bit[64:0] o_double_slice, output bit[127:0] o_wide___0, output bit[127:0] o_wide___1);

    chandle ___instance___ = null;

    initial begin
        ___instance___ = mux_new();
        if (___instance___ == null) begin
            $fatal(0, "Failed to create mux instance.");
        end
    end

    final begin
        if (___instance___ !== null) begin
            mux_free(___instance___);
            ___instance___ = null;
        end
    end

    always @(posedge i_clk) begin
        if (mux_tick(___instance___, i_clk, i_rst, i_sel, i_a, i_b, i_double, i_u16_array[0], i_u16_array[1], i_u16_array[2], i_u16_array[3], i_u16_array[4], i_u16_array[5], i_u16_array[6], i_u16_array[7], o_aorb, o_slice, o_double_slice, o_wide[0], o_wide[1]) != 0) begin
            $fatal(0, "Failed to tick mux instance.");
        end
    end
    