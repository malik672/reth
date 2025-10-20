
================================================================================
REAL-WORLD DATASET (1000 keys, 10-64 nibbles, realistic hash distribution)
================================================================================

__ZN14prefix_set_asm18contains_realworld17ha4a200a56572c149E:
Lfunc_begin0:
	.cfi_startproc
	sub	sp, sp, #64
	.cfi_def_cfa_offset 64
	stp	x20, x19, [sp, #32]
	stp	x29, x30, [sp, #48]
	add	x29, sp, #48
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	.cfi_remember_state
	ldrb	w8, [x0, #16]
	tbz	w8, #0, LBB0_2
	mov	w0, #1
	b	LBB0_31
LBB0_2:
	ldp	x11, x8, [x0]
	cbz	x8, LBB0_25
	mov	x13, x1
	ldr	x10, [x13], #39
	ldr	x9, [x11, #32]
	sub	x14, x10, x10, lsr #1
	mov	w15, #40
	mov	x12, #39
	madd	x16, x8, x15, x12
	mov	w17, #32
	b	LBB0_6
LBB0_4:
	cmp	x2, x10
	b.ls	LBB0_12
LBB0_5:
	sub	x16, x16, #40
	sub	x8, x8, #1
	str	x8, [x0, #8]
	cbz	x8, LBB0_12
LBB0_6:
	cmp	x8, x9
	b.hs	LBB0_34
	ldr	x12, [x11, #24]
	mul	x2, x8, x15
	ldr	x2, [x12, x2]
	sub	x3, x2, x2, lsr #1
	cmp	x14, x3
	csel	x3, x14, x3, lo
	cmp	x3, #32
	b.hi	LBB0_32
	mov	x4, x16
	mov	x5, x13
LBB0_9:
	cbz	x3, LBB0_4
	sub	x3, x3, #1
	ldrb	w6, [x12, x4]
	ldrb	w7, [x5], #-1
	sub	x4, x4, #1
	cmp	w6, w7
	b.eq	LBB0_9
	b.hi	LBB0_5
LBB0_12:
	cbz	x10, LBB0_26
LBB0_13:
	cmp	x8, x9
	b.eq	LBB0_27
	mov	x11, #0
	mov	w13, #40
	madd	x14, x8, x13, x12
	madd	x9, x9, x13, x12
Lloh0:
	adrp	x12, __ZN7nybbles7nibbles11SLICE_MASKS17h8011fa7f13f3d1b9E@GOTPAGE
Lloh1:
	ldr	x12, [x12, __ZN7nybbles7nibbles11SLICE_MASKS17h8011fa7f13f3d1b9E@GOTPAGEOFF]
	add	x12, x12, x10, lsl #5
	sub	x13, x10, x10, lsr #1
	mov	w15, #32
	b	LBB0_17
LBB0_15:
	cmp	x16, x10
	b.hi	LBB0_24
LBB0_16:
	mov	x14, x2
	mov	x11, x17
	cmp	x2, x9
	b.eq	LBB0_27
LBB0_17:
	ldr	x16, [x14]
	cmp	x10, x16
	b.hi	LBB0_19
	ldur	q0, [x14, #8]
	ldp	q1, q2, [x12]
	and.16b	v0, v0, v1
	ldur	q1, [x14, #24]
	and.16b	v1, v1, v2
	stp	q0, q1, [sp]
	ldp	x17, x2, [sp]
	ldp	x3, x4, [x1, #8]
	ldp	x5, x6, [sp, #16]
	ldp	x7, x19, [x1, #24]
	cmp	x17, x3
	ccmp	x2, x4, #0, eq
	ccmp	x5, x7, #0, eq
	ccmp	x6, x19, #0, eq
	b.eq	LBB0_29
LBB0_19:
	sub	x17, x16, x16, lsr #1
	cmp	x13, x17
	csel	x3, x13, x17, lo
	cmp	x3, #32
	b.hi	LBB0_33
	add	x17, x11, #1
	add	x2, x14, #40
	mov	w4, #39
LBB0_21:
	add	x5, x3, x4
	cmp	x5, #39
	b.eq	LBB0_15
	ldrb	w5, [x14, x4]
	ldrb	w6, [x1, x4]
	sub	x4, x4, #1
	cmp	w5, w6
	b.eq	LBB0_21
	b.ls	LBB0_16
LBB0_24:
	mov	w9, #0
	b	LBB0_30
LBB0_25:
	ldp	x12, x9, [x11, #24]
	ldr	x10, [x1]
	cbnz	x10, LBB0_13
LBB0_26:
	cmp	x8, x9
	b.ne	LBB0_28
LBB0_27:
	mov	w0, #0
	b	LBB0_31
LBB0_28:
	mov	x11, #0
LBB0_29:
	mov	w9, #1
LBB0_30:
	add	x8, x11, x8
	str	x8, [x0, #8]
	mov	x0, x9
LBB0_31:
	.cfi_def_cfa wsp, 64
	ldp	x29, x30, [sp, #48]
	ldp	x20, x19, [sp, #32]
	add	sp, sp, #64
	.cfi_def_cfa_offset 0
	.cfi_restore w30
	.cfi_restore w29
	.cfi_restore w19
	.cfi_restore w20
	ret