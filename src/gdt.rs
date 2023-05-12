use x86_64::{VirtAddr, structures::gdt::SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use lazy_static::lazy_static;

//// this is LITERALLY just a safe wrapper around the single x86_64 instruction lgdt.
//// it's the same thing as doing it raw in assembler.
// pub unsafe fn load_unsafe(&self) {
//     use crate::instructions::tables::lgdt;
//     unsafe {
//         lgdt(&self.pointer());
//     }
// }
pub fn init()
{
    use x86_64::instructions::tables::load_tss;
    use x86_64::instructions::segmentation::{CS, Segment};

    GDT.0.load();
    // every time we change the GDT, we have to refresh it.
    unsafe {
        // reset the code segment pointer.
        CS::set_reg(GDT.1.code_selector);
        // load the Task State Segment structure pointer, and start actually using it.
        load_tss(GDT.1.tss_selector);
    }
}

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

// we need multiple stacks to handle exceptions.
// when an exception occurs, a stack trace is pushed out onto the default stack
// if the exception was a stack overflow, it'll double fault.
// that double fault pushes its own stack frame, then it triple faults x_x

// so, x86_64 has a second, for-sure-safe stack that we can always push our stack frames to, called the Interrupt Stack Table.
// more than two. the IST is actually seven pointers to known-good stacks that can be used to recover from kernel SO.

//// PART OF THE TASK STATE SEGMENT STRUCT:
    // the privilege stack table is related to rings of permission.
    // it allows kernel and userspace applications to use different stacks.
    // this is probably a good thing; you wouldn't want a user app SOing the kernel and etc.
    // nor would you want the user to have direct access to kernel stack variables.
// pub privilege_stack_table: [VirtAddr; 3],
// reserved_2: u64,
// /// The full 64-bit canonical forms of the interrupt stack table (IST) pointers.
    // the IST we want to make.
// pub interrupt_stack_table: [VirtAddr; 7],

// so we make a tss, then that's our stack reference.
// like most tables, this doesn't need to be mutable either, since it just holds a bunch of memory addresses.
lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            // make the 0th entry the DF stack, so that our DFs are safe from kernel SO.
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe {&STACK});
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss
    };
}

//// so as usual, there's quite a bit of nesting going on here.
//// let's recap:
// the stack tables are in the task state segment
// IST & PST -> TSS -> GDT
// then we store that SEGMENT into the x86 segment table, the GDT or global descriptor table.
// so now, we define then load our GDT.

use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};

// again, just a non-mutable reference. these are all nested pointer tables.
// doing nothing fancy, just magic words by this point.
lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        // gdt is a list of u64.
        // hmm
        // where have i heard that number before
        let mut gdt = GlobalDescriptorTable::new();
        // we need to reset up the kernel code segment that was already working.
        // remember, we're overriding a default behavior here.
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors {code_selector, tss_selector})
    };
}

struct Selectors
{
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}
