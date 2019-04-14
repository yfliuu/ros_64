# Initial process execs /init.
# This code runs in user space.


# exec(init, argv)
.globl user_start
user_start:
  mov $init, %rdi
  mov $argv, %rsi
  mov $7, %rax
  int $64

# for(;;) exit();
exit:
  mov $2, %rax
  int $64
  jmp exit

# char init[] = "/init\0";
init:
  .string "/init\0"

# char *argv[] = { init, 0 };
.p2align 2
argv:
  .quad init
  .quad 0