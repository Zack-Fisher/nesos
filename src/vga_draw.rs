const SCREEN_WIDTH: usize = 320;
const SCREEN_HEIGHT: usize = 200;
const VGA_ADDRESS: *mut u8 = 0xA0000 as *mut u8;

pub unsafe fn set_pixel(x: isize, y: isize, color: u8) {
    if x >= 0 && x < (SCREEN_WIDTH as isize) && y >= 0 && y < (SCREEN_HEIGHT as isize) {
        core::ptr::write_volatile(VGA_ADDRESS.offset(y * (SCREEN_WIDTH as isize) + x), color);
    }
}

pub unsafe fn draw_line(input_x0: isize, input_y0: isize, x1: isize, y1: isize, color: u8) {
    let mut x0 = input_x0;
    let mut y0 = input_y0;

    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };

    let mut err = if dx > dy { dx } else { -dy } / 2;
    let mut e2;

    loop {
        set_pixel(x0, y0, color);

        if x0 == x1 && y0 == y1 {
            break;
        }

        e2 = err;

        if e2 > -dx {
            err -= dy;
            x0 += sx;
        }
        if e2 < dy {
            err += dx;
            y0 += sy;
        }
    }
}