option casemap:none

EXTERN gore_as_capture_production_shim_before:PROC
EXTERN gore_as_capture_production_shim_after:PROC
EXTERN gore_as_capture_production_shim_targets:QWORD

FRAME_BASE EQU 020h
FRAME_RAX EQU FRAME_BASE + 000h
FRAME_RCX EQU FRAME_BASE + 008h
FRAME_RDX EQU FRAME_BASE + 010h
FRAME_RBX EQU FRAME_BASE + 018h
FRAME_RSP EQU FRAME_BASE + 020h
FRAME_RBP EQU FRAME_BASE + 028h
FRAME_RSI EQU FRAME_BASE + 030h
FRAME_RDI EQU FRAME_BASE + 038h
FRAME_R8  EQU FRAME_BASE + 040h
FRAME_R9  EQU FRAME_BASE + 048h
FRAME_R10 EQU FRAME_BASE + 050h
FRAME_R11 EQU FRAME_BASE + 058h
FRAME_R12 EQU FRAME_BASE + 060h
FRAME_R13 EQU FRAME_BASE + 068h
FRAME_R14 EQU FRAME_BASE + 070h
FRAME_R15 EQU FRAME_BASE + 078h
FRAME_RFLAGS EQU FRAME_BASE + 080h
FRAME_XMM EQU FRAME_BASE + 090h

; PUSHFQ must precede SUB because SUB changes the arithmetic flags being observed. A function/
; CALL entry (RSP%16=8) pushes flags then subtracts 1d0h; an inline entry (RSP%16=0) pushes flags
; then subtracts 1d8h. Both dispatcher stacks are aligned. UNWIND_INFO describes the total stack
; displacement (PUSHFQ plus SUB); PUSHFQ cannot fault and no call occurs in the partial prolog.

SAVE_STATE MACRO stack_sub, stack_total
  pushfq
  sub rsp, stack_sub
  .allocstack stack_total
  mov [rsp + FRAME_RBX], rbx
  .savereg rbx, FRAME_RBX
  mov [rsp + FRAME_RBP], rbp
  .savereg rbp, FRAME_RBP
  mov [rsp + FRAME_RSI], rsi
  .savereg rsi, FRAME_RSI
  mov [rsp + FRAME_RDI], rdi
  .savereg rdi, FRAME_RDI
  mov [rsp + FRAME_R12], r12
  .savereg r12, FRAME_R12
  mov [rsp + FRAME_R13], r13
  .savereg r13, FRAME_R13
  mov [rsp + FRAME_R14], r14
  .savereg r14, FRAME_R14
  mov [rsp + FRAME_R15], r15
  .savereg r15, FRAME_R15
  movdqu xmmword ptr [rsp + FRAME_XMM + 060h], xmm6
  .savexmm128 xmm6, FRAME_XMM + 060h
  movdqu xmmword ptr [rsp + FRAME_XMM + 070h], xmm7
  .savexmm128 xmm7, FRAME_XMM + 070h
  movdqu xmmword ptr [rsp + FRAME_XMM + 080h], xmm8
  .savexmm128 xmm8, FRAME_XMM + 080h
  movdqu xmmword ptr [rsp + FRAME_XMM + 090h], xmm9
  .savexmm128 xmm9, FRAME_XMM + 090h
  movdqu xmmword ptr [rsp + FRAME_XMM + 0a0h], xmm10
  .savexmm128 xmm10, FRAME_XMM + 0a0h
  movdqu xmmword ptr [rsp + FRAME_XMM + 0b0h], xmm11
  .savexmm128 xmm11, FRAME_XMM + 0b0h
  movdqu xmmword ptr [rsp + FRAME_XMM + 0c0h], xmm12
  .savexmm128 xmm12, FRAME_XMM + 0c0h
  movdqu xmmword ptr [rsp + FRAME_XMM + 0d0h], xmm13
  .savexmm128 xmm13, FRAME_XMM + 0d0h
  movdqu xmmword ptr [rsp + FRAME_XMM + 0e0h], xmm14
  .savexmm128 xmm14, FRAME_XMM + 0e0h
  movdqu xmmword ptr [rsp + FRAME_XMM + 0f0h], xmm15
  .savexmm128 xmm15, FRAME_XMM + 0f0h
  .endprolog
  ; Volatile state is not part of Windows unwind restoration. Keeping it outside the encoded
  ; prolog holds PrologSize below 256 while the complete observer state is still preserved.
  mov [rsp + FRAME_RAX], rax
  mov [rsp + FRAME_RCX], rcx
  mov [rsp + FRAME_RDX], rdx
  lea rax, [rsp + stack_total]
  mov [rsp + FRAME_RSP], rax
  mov [rsp + FRAME_R8], r8
  mov [rsp + FRAME_R9], r9
  mov [rsp + FRAME_R10], r10
  mov [rsp + FRAME_R11], r11
  mov rax, [rsp + stack_sub]
  mov [rsp + FRAME_RFLAGS], rax
  movdqu xmmword ptr [rsp + FRAME_XMM + 000h], xmm0
  movdqu xmmword ptr [rsp + FRAME_XMM + 010h], xmm1
  movdqu xmmword ptr [rsp + FRAME_XMM + 020h], xmm2
  movdqu xmmword ptr [rsp + FRAME_XMM + 030h], xmm3
  movdqu xmmword ptr [rsp + FRAME_XMM + 040h], xmm4
  movdqu xmmword ptr [rsp + FRAME_XMM + 050h], xmm5
ENDM

RESTORE_STATE MACRO stack_total
  movdqu xmm0, xmmword ptr [rsp + FRAME_XMM + 000h]
  movdqu xmm1, xmmword ptr [rsp + FRAME_XMM + 010h]
  movdqu xmm2, xmmword ptr [rsp + FRAME_XMM + 020h]
  movdqu xmm3, xmmword ptr [rsp + FRAME_XMM + 030h]
  movdqu xmm4, xmmword ptr [rsp + FRAME_XMM + 040h]
  movdqu xmm5, xmmword ptr [rsp + FRAME_XMM + 050h]
  movdqu xmm6, xmmword ptr [rsp + FRAME_XMM + 060h]
  movdqu xmm7, xmmword ptr [rsp + FRAME_XMM + 070h]
  movdqu xmm8, xmmword ptr [rsp + FRAME_XMM + 080h]
  movdqu xmm9, xmmword ptr [rsp + FRAME_XMM + 090h]
  movdqu xmm10, xmmword ptr [rsp + FRAME_XMM + 0a0h]
  movdqu xmm11, xmmword ptr [rsp + FRAME_XMM + 0b0h]
  movdqu xmm12, xmmword ptr [rsp + FRAME_XMM + 0c0h]
  movdqu xmm13, xmmword ptr [rsp + FRAME_XMM + 0d0h]
  movdqu xmm14, xmmword ptr [rsp + FRAME_XMM + 0e0h]
  movdqu xmm15, xmmword ptr [rsp + FRAME_XMM + 0f0h]
  mov rcx, [rsp + FRAME_RCX]
  mov rdx, [rsp + FRAME_RDX]
  mov rbx, [rsp + FRAME_RBX]
  mov rbp, [rsp + FRAME_RBP]
  mov rsi, [rsp + FRAME_RSI]
  mov rdi, [rsp + FRAME_RDI]
  mov r8, [rsp + FRAME_R8]
  mov r9, [rsp + FRAME_R9]
  mov r10, [rsp + FRAME_R10]
  mov r11, [rsp + FRAME_R11]
  mov r12, [rsp + FRAME_R12]
  mov r13, [rsp + FRAME_R13]
  mov r14, [rsp + FRAME_R14]
  mov r15, [rsp + FRAME_R15]
  push qword ptr [rsp + FRAME_RFLAGS]
  popfq
  mov rax, [rsp + FRAME_RAX]
  lea rsp, [rsp + stack_total]
ENDM

ENTRY_SHIM MACRO symbol, site_id, stack_sub, stack_total
symbol PROC FRAME
  SAVE_STATE stack_sub, stack_total
  lea rcx, [rsp + FRAME_BASE]
  mov edx, site_id
  call gore_as_capture_production_shim_before
  RESTORE_STATE stack_total
  jmp qword ptr [gore_as_capture_production_shim_targets + site_id * 8]
symbol ENDP
ENDM

.code

ENTRY_SHIM gore_as_capture_production_site_00, 00, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_01, 01, 01d8h, 01e0h
ENTRY_SHIM gore_as_capture_production_site_02, 02, 01d8h, 01e0h
ENTRY_SHIM gore_as_capture_production_site_03, 03, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_04, 04, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_05, 05, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_06, 06, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_07, 07, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_08, 08, 01d8h, 01e0h

ENTRY_SHIM gore_as_capture_production_site_09, 09, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_10, 10, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_11, 11, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_12, 12, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_13, 13, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_14, 14, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_15, 15, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_16, 16, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_17, 17, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_18, 18, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_19, 19, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_20, 20, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_21, 21, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_22, 22, 01d0h, 01d8h

ENTRY_SHIM gore_as_capture_production_site_23, 23, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_24, 24, 01d0h, 01d8h
ENTRY_SHIM gore_as_capture_production_site_25, 25, 01d0h, 01d8h

gore_as_capture_production_return PROC FRAME
  SAVE_STATE 01d8h, 01e0h
  lea rcx, [rsp + FRAME_BASE]
  call gore_as_capture_production_shim_after
  RESTORE_STATE 01e0h
  lea rsp, [rsp - 8]
  ret
gore_as_capture_production_return ENDP

gore_as_capture_production_fixture_target PROC
  ret
gore_as_capture_production_fixture_target ENDP

; Executes one complete function-entry shim with every GPR and XMM register populated. The
; fixture target is a single RET, so any mismatch is attributable to the shim save/restore path.
gore_as_capture_production_shim_state_selftest PROC FRAME
  push rbx
  .pushreg rbx
  push rbp
  .pushreg rbp
  push rsi
  .pushreg rsi
  push rdi
  .pushreg rdi
  push r12
  .pushreg r12
  push r13
  .pushreg r13
  push r14
  .pushreg r14
  push r15
  .pushreg r15
  sub rsp, 0c8h
  .allocstack 0c8h
  movdqu xmmword ptr [rsp + 020h], xmm6
  .savexmm128 xmm6, 020h
  movdqu xmmword ptr [rsp + 030h], xmm7
  .savexmm128 xmm7, 030h
  movdqu xmmword ptr [rsp + 040h], xmm8
  .savexmm128 xmm8, 040h
  movdqu xmmword ptr [rsp + 050h], xmm9
  .savexmm128 xmm9, 050h
  movdqu xmmword ptr [rsp + 060h], xmm10
  .savexmm128 xmm10, 060h
  movdqu xmmword ptr [rsp + 070h], xmm11
  .savexmm128 xmm11, 070h
  movdqu xmmword ptr [rsp + 080h], xmm12
  .savexmm128 xmm12, 080h
  movdqu xmmword ptr [rsp + 090h], xmm13
  .savexmm128 xmm13, 090h
  movdqu xmmword ptr [rsp + 0a0h], xmm14
  .savexmm128 xmm14, 0a0h
  movdqu xmmword ptr [rsp + 0b0h], xmm15
  .savexmm128 xmm15, 0b0h
  .endprolog

  mov rax, qword ptr [gore_as_capture_production_shim_targets]
  mov [rsp + 00h], rax
  lea rax, gore_as_capture_production_fixture_target
  mov qword ptr [gore_as_capture_production_shim_targets], rax
  mov [rsp + 018h], rsp
  mov dword ptr [rsp + 0c0h], 2

  movdqu xmm0, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm1, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm2, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm3, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm4, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm5, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm6, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm7, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm8, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm9, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm10, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm11, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm12, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm13, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm14, xmmword ptr [gore_as_capture_production_fixture_xmm]
  movdqu xmm15, xmmword ptr [gore_as_capture_production_fixture_xmm]
  push 0202h
  popfq
  pushfq
  pop qword ptr [rsp + 008h]

  mov rax, 0101h
  mov rcx, 0202h
  mov rdx, 0303h
  mov rbx, 0404h
  mov rbp, 0505h
  mov rsi, 0606h
  mov rdi, 0707h
  mov r8, 0808h
  mov r9, 0909h
  mov r10, 0a0ah
  mov r11, 0b0bh
  mov r12, 0c0ch
  mov r13, 0d0dh
  mov r14, 0e0eh
  mov r15, 0f0fh
  call gore_as_capture_production_site_00

  pushfq
  pop qword ptr [rsp + 010h]
  cmp rax, 0101h
  jne fixture_failed
  cmp rcx, 0202h
  jne fixture_failed
  cmp rdx, 0303h
  jne fixture_failed
  cmp rbx, 0404h
  jne fixture_failed
  cmp rbp, 0505h
  jne fixture_failed
  cmp rsi, 0606h
  jne fixture_failed
  cmp rdi, 0707h
  jne fixture_failed
  cmp r8, 0808h
  jne fixture_failed
  cmp r9, 0909h
  jne fixture_failed
  cmp r10, 0a0ah
  jne fixture_failed
  cmp r11, 0b0bh
  jne fixture_failed
  cmp r12, 0c0ch
  jne fixture_failed
  cmp r13, 0d0dh
  jne fixture_failed
  cmp r14, 0e0eh
  jne fixture_failed
  cmp r15, 0f0fh
  jne fixture_failed
  cmp rsp, [rsp + 018h]
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 3
  ; Intel reserves RFLAGS[63:32]; PUSHFQ may expose environment-dependent reserved contents which
  ; POPFQ neither consumes nor promises to reproduce. Compare every architecturally defined bit.
  mov eax, dword ptr [rsp + 008h]
  cmp eax, dword ptr [rsp + 010h]
  jne fixture_flags_failed

  mov dword ptr [rsp + 0c0h], 4

  mov dword ptr [rsp + 0c0h], 040h
  pcmpeqb xmm0, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm0
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 041h
  pcmpeqb xmm1, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm1
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 042h
  pcmpeqb xmm2, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm2
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 043h
  pcmpeqb xmm3, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm3
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 044h
  pcmpeqb xmm4, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm4
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 045h
  pcmpeqb xmm5, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm5
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 046h
  pcmpeqb xmm6, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm6
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 047h
  pcmpeqb xmm7, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm7
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 048h
  pcmpeqb xmm8, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm8
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 049h
  pcmpeqb xmm9, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm9
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 04ah
  pcmpeqb xmm10, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm10
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 04bh
  pcmpeqb xmm11, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm11
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 04ch
  pcmpeqb xmm12, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm12
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 04dh
  pcmpeqb xmm13, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm13
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 04eh
  pcmpeqb xmm14, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm14
  cmp eax, 0ffffh
  jne fixture_failed
  mov dword ptr [rsp + 0c0h], 04fh
  pcmpeqb xmm15, xmmword ptr [gore_as_capture_production_fixture_xmm]
  pmovmskb eax, xmm15
  cmp eax, 0ffffh
  jne fixture_failed
  mov eax, 1
  jmp fixture_restore
fixture_failed:
  mov eax, dword ptr [rsp + 0c0h]
  jmp fixture_restore
fixture_flags_failed:
  xor eax, dword ptr [rsp + 010h]
  or eax, 10000h
fixture_restore:
  mov rcx, [rsp + 00h]
  mov qword ptr [gore_as_capture_production_shim_targets], rcx
  movdqu xmm6, xmmword ptr [rsp + 020h]
  movdqu xmm7, xmmword ptr [rsp + 030h]
  movdqu xmm8, xmmword ptr [rsp + 040h]
  movdqu xmm9, xmmword ptr [rsp + 050h]
  movdqu xmm10, xmmword ptr [rsp + 060h]
  movdqu xmm11, xmmword ptr [rsp + 070h]
  movdqu xmm12, xmmword ptr [rsp + 080h]
  movdqu xmm13, xmmword ptr [rsp + 090h]
  movdqu xmm14, xmmword ptr [rsp + 0a0h]
  movdqu xmm15, xmmword ptr [rsp + 0b0h]
  add rsp, 0c8h
  pop r15
  pop r14
  pop r13
  pop r12
  pop rdi
  pop rsi
  pop rbp
  pop rbx
  ret
gore_as_capture_production_shim_state_selftest ENDP

.const
ALIGN 16
gore_as_capture_production_fixture_xmm BYTE \
  00h, 11h, 22h, 33h, 44h, 55h, 66h, 77h, \
  88h, 99h, 0aah, 0bbh, 0cch, 0ddh, 0eeh, 0ffh

END
