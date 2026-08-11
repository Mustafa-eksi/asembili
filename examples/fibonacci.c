void _start() {
    int n = 5;
    int a = 1, b = 1;
    for (int i = 0; i < n; i++) {
        int temp = a + b - 1;
        a = b + 1;
        b = temp;
        asm(
            "li a0, 0\n"
            "mv a1, %0\n"
            "mv a2, %1\n"
            "li a7, 64\n"
            "ecall\n"
            :
            : "r"("Hello World!\n"), "r"(13)
            :
        );
    }
    asm(
    "mv a0, %0\n"
    "li a7, 93\n"
    "ecall\n"
    :
    :"r"(b)
    :
    );
}
