// ==== FUN_100010b0 @ 100010b0 size=181

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_100010b0(void)

{
  int iVar1;
  int *piVar2;
  ushort in_DS;
  int unaff_FS_OFFSET;
  
  iVar1 = FUN_10002606();
  if (iVar1 != 0) {
    if ((in_DS & 4) == 0) {
      iVar1 = FUN_1000260c();
      if (iVar1 != 0) {
LAB_1000113d:
        DAT_10002680 = iVar1;
        iVar1 = FUN_1000260c();
        if ((iVar1 != 0) && (_DAT_10002688 = iVar1, iVar1 = FUN_1000260c(), iVar1 != 0)) {
          DAT_10002684 = iVar1;
          return 1;
        }
      }
    }
    else {
      piVar2 = (int *)FUN_1000260c();
      if (piVar2 != (int *)0x0) {
        iVar1 = 0x10000;
        do {
          if (((*piVar2 == -0xfff98) && ((char)piVar2[1] == -1)) &&
             (*(char *)((int)piVar2 + 6) == -0x18)) {
            DAT_10002678 = (int)piVar2 + *(int *)((int)piVar2 + 7) + 0xb;
            _DAT_1000267c = FUN_10002612();
            _DAT_1000267c = _DAT_1000267c ^ *(uint *)(unaff_FS_OFFSET + 0x30);
            s_VirtualAllocEx_10001078[0xc] = '\0';
            s_VirtualFreeEx_10001087[0xb] = '\0';
            iVar1 = DAT_10002680;
            goto LAB_1000113d;
          }
          piVar2 = (int *)((int)piVar2 + 1);
          iVar1 = iVar1 + -1;
        } while (iVar1 != 0);
      }
    }
  }
  return 0;
}


// ==== FUN_10001165 @ 10001165 size=47

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10001165(void)

{
  (*_DAT_10002688)();
  return;
}


// ==== FUN_10001194 @ 10001194 size=34

void FUN_10001194(void)

{
  (*DAT_10002684)();
  return;
}


// ==== FUN_100011b6 @ 100011b6 size=161

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

undefined4 FUN_100011b6(undefined4 param_1,uint param_2,int param_3,undefined4 param_4,int param_5)

{
  int *piVar1;
  undefined4 uVar2;
  ushort in_DS;
  
  if ((in_DS & 4) == 0) {
    uVar2 = 0;
    if (param_5 != 0) {
      uVar2 = 4;
    }
    uVar2 = (*DAT_10002680)(param_1,0,0,param_3,param_4,uVar2,&stack0xfffffffc);
  }
  else {
    if ((*(uint *)((param_2 ^ _DAT_1000267c) + 0x20) & 0x30800008) != 0) {
      return 0;
    }
    piVar1 = (int *)(*DAT_10002678)(param_2 ^ _DAT_1000267c,0xffffc000,param_3,param_4);
    uVar2 = 0;
    if (piVar1 != (int *)0x0) {
      *piVar1 = *piVar1 + -1;
      uVar2 = FUN_10002600();
      iRam001f0fff = iRam001f0fff + 1;
      if (param_3 == 0) {
        FUN_100025fa();
      }
      return uVar2;
    }
  }
  return uVar2;
}


// ==== FUN_10001258 @ 10001258 size=17

undefined8 __fastcall FUN_10001258(undefined4 param_1,undefined4 param_2)

{
  uint *puVar1;
  uint uVar2;
  uint uVar3;
  undefined4 in_EAX;
  int iVar4;
  int unaff_EBX;
  uint *unaff_ESI;
  int *unaff_EDI;
  int *piVar5;
  bool bVar6;
  
  iVar4 = 0x80;
  bVar6 = false;
  piVar5 = unaff_EDI;
  do {
    puVar1 = (uint *)((int)piVar5 + (unaff_EBX - (int)unaff_EDI));
    uVar2 = (uint)bVar6;
    uVar3 = *unaff_ESI - *puVar1;
    bVar6 = *unaff_ESI < *puVar1 || uVar3 < uVar2;
    *piVar5 = uVar3 - uVar2;
    iVar4 = iVar4 + -1;
    unaff_ESI = unaff_ESI + 1;
    piVar5 = piVar5 + 1;
  } while (iVar4 != 0);
  return CONCAT44(param_2,in_EAX);
}


// ==== FUN_10001269 @ 10001269 size=27

int FUN_10001269(void)

{
  int in_EAX;
  int iVar1;
  int unaff_EBP;
  int *piVar2;
  int *piVar3;
  bool bVar4;
  
  iVar1 = 0;
  bVar4 = true;
  piVar2 = (int *)(in_EAX + -4 + unaff_EBP * 4);
  do {
    piVar3 = piVar2;
    if (unaff_EBP == 0) break;
    unaff_EBP = unaff_EBP + -1;
    piVar3 = piVar2 + -1;
    bVar4 = *piVar2 == 0;
    piVar2 = piVar3;
  } while (bVar4);
  if (!bVar4) {
    iVar1 = 0x1f;
    if (piVar3[1] != 0) {
      for (; (uint)piVar3[1] >> iVar1 == 0; iVar1 = iVar1 + -1) {
      }
    }
    iVar1 = iVar1 + 1;
  }
  return iVar1;
}


// ==== FUN_10001284 @ 10001284 size=144

undefined8 __fastcall FUN_10001284(undefined4 param_1,undefined4 param_2)

{
  uint uVar1;
  uint uVar2;
  uint uVar3;
  longlong lVar4;
  undefined4 in_EAX;
  uint uVar5;
  char cVar10;
  int iVar6;
  int extraout_ECX;
  uint uVar7;
  uint uVar8;
  uint uVar9;
  uint extraout_EDX;
  uint *unaff_EBX;
  uint *puVar11;
  uint uVar12;
  uint uVar13;
  uint *unaff_ESI;
  uint *puVar14;
  uint *puVar15;
  undefined4 *unaff_EDI;
  uint *puVar16;
  bool bVar17;
  bool bVar18;
  uint local_220 [128];
  
  bVar18 = false;
  uVar13 = 0x80;
  puVar15 = local_220;
  puVar11 = local_220;
  for (iVar6 = 0x80; iVar6 != 0; iVar6 = iVar6 + -1) {
    *puVar11 = 0;
    puVar11 = puVar11 + 1;
  }
  FUN_10001269();
  FUN_10001269();
  uVar7 = extraout_ECX + 0x1fU >> 5;
  uVar8 = uVar7;
  uVar9 = extraout_EDX;
  puVar11 = unaff_EBX;
  if (extraout_EDX < uVar7) {
    uVar8 = extraout_EDX;
    uVar9 = uVar7;
    puVar11 = unaff_ESI;
    unaff_ESI = unaff_EBX;
  }
  uVar8 = CONCAT11((char)uVar8,(char)uVar9) + 1;
  cVar10 = (char)(uVar8 >> 8);
  while (cVar10 != '\0') {
    uVar9 = uVar8 & 0xff;
    if (uVar13 < (uVar8 & 0xff)) {
      uVar9 = uVar13;
    }
    uVar7 = *puVar11;
    uVar12 = 0;
    bVar17 = false;
    puVar14 = unaff_ESI;
    puVar16 = puVar15;
    do {
      uVar1 = (uint)bVar18;
      lVar4 = (ulonglong)*puVar14 * (ulonglong)uVar7 + (ulonglong)uVar12;
      uVar5 = (uint)lVar4;
      uVar12 = (uint)((ulonglong)lVar4 >> 0x20);
      bVar18 = bVar18 != false;
      uVar2 = (uint)bVar17;
      uVar3 = uVar5 + *puVar16;
      bVar17 = CARRY4(uVar5,*puVar16) || CARRY4(uVar3,uVar2);
      *puVar16 = uVar3 + uVar2;
      uVar9 = uVar9 - 1;
      puVar14 = puVar14 + uVar1 * -2 + 1;
      puVar16 = puVar16 + (uint)bVar18 * -2 + 1;
    } while (uVar9 != 0);
    puVar11 = puVar11 + 1;
    puVar15 = puVar15 + 1;
    uVar13 = uVar13 - 1;
    cVar10 = (char)(uVar8 >> 8) + -1;
    uVar8 = (uint)CONCAT11(cVar10,(char)uVar8);
  }
  puVar15 = local_220;
  for (iVar6 = 0x80; iVar6 != 0; iVar6 = iVar6 + -1) {
    *unaff_EDI = *puVar15;
    puVar15 = puVar15 + (uint)bVar18 * -2 + 1;
    unaff_EDI = unaff_EDI + (uint)bVar18 * -2 + 1;
  }
  return CONCAT44(param_2,in_EAX);
}


// ==== FUN_10001314 @ 10001314 size=24

undefined8 __fastcall FUN_10001314(undefined4 param_1,int param_2)

{
  int iVar1;
  undefined4 in_EAX;
  int iVar2;
  int *piVar3;
  int unaff_ESI;
  int *piVar4;
  
  iVar2 = 0x80;
  piVar3 = (int *)(param_2 + 0x200);
  piVar4 = (int *)(unaff_ESI + 0x1fc);
  do {
    iVar1 = *piVar4;
    piVar3 = piVar3 + -1;
    iVar2 = iVar2 + -1;
    piVar4 = piVar4 + -1;
  } while (iVar2 != 0 && iVar1 == *piVar3);
  return CONCAT44(param_2,in_EAX);
}


// ==== FUN_1000132c @ 1000132c size=125

undefined8 __fastcall FUN_1000132c(undefined4 param_1,undefined4 param_2)

{
  uint uVar1;
  byte bVar2;
  undefined4 in_EAX;
  int extraout_ECX;
  int extraout_ECX_00;
  uint uVar3;
  uint uVar4;
  uint uVar5;
  int extraout_ECX_01;
  int extraout_ECX_02;
  int iVar6;
  int extraout_EDX;
  uint *puVar7;
  uint *unaff_EBX;
  int iVar8;
  uint *puVar9;
  bool bVar10;
  undefined8 uVar11;
  uint auStack_220 [128];
  
  bVar10 = &stack0xffffffe0 < (undefined1 *)0x200;
  FUN_10001314(param_1,(int)unaff_EBX);
  if (!bVar10) {
    FUN_10001269();
    if (extraout_ECX != 0) {
      FUN_10001269();
      uVar3 = extraout_ECX_00 - extraout_EDX;
      uVar4 = uVar3 >> 5;
      uVar5 = uVar4;
      puVar9 = auStack_220;
      puVar7 = auStack_220;
      if (uVar4 != 0) {
        for (; puVar7 = puVar9, uVar5 != 0; uVar5 = uVar5 - 1) {
          *puVar9 = 0;
          puVar9 = puVar9 + 1;
        }
      }
      uVar5 = 0;
      for (; uVar4 < 0x80; uVar4 = uVar4 + 1) {
        uVar1 = *unaff_EBX;
        bVar2 = (byte)uVar3 & 0x1f;
        *puVar7 = uVar1 << bVar2 | uVar5 >> 0x20 - bVar2;
        uVar5 = uVar1;
        unaff_EBX = unaff_EBX + 1;
        puVar7 = puVar7 + 1;
      }
      bVar10 = false;
      do {
        uVar11 = FUN_10001314(uVar3,(int)auStack_220);
        iVar8 = (int)((ulonglong)uVar11 >> 0x20);
        iVar6 = extraout_ECX_01;
        if (!bVar10) {
          uVar11 = FUN_10001258(extraout_ECX_01,iVar8);
          iVar8 = (int)((ulonglong)uVar11 >> 0x20);
          iVar6 = extraout_ECX_02;
        }
        puVar7 = (uint *)(iVar8 + 0x1fc);
        bVar10 = false;
        iVar8 = 0x80;
        do {
          uVar5 = *puVar7;
          *puVar7 = (uint)(CONCAT14(bVar10,uVar5) >> 1);
          bVar10 = (uVar5 & 1) != 0;
          puVar7 = puVar7 + -1;
          iVar8 = iVar8 + -1;
        } while (iVar8 != 0);
        uVar3 = iVar6 - 1;
      } while (-1 < (int)uVar3);
    }
  }
  return CONCAT44(param_2,in_EAX);
}


// ==== FUN_100013a9 @ 100013a9 size=56

/* WARNING: Control flow encountered bad instruction data */

void FUN_100013a9(void)

{
  int iVar1;
  undefined4 extraout_EDX;
  undefined4 *unaff_ESI;
  undefined4 *puVar2;
  undefined4 *puVar3;
  undefined8 uVar4;
  undefined4 auStack_630 [388];
  
  FUN_10001516();
  puVar3 = auStack_630;
  for (iVar1 = 0x40; iVar1 != 0; iVar1 = iVar1 + -1) {
    *puVar3 = *unaff_ESI;
    unaff_ESI = unaff_ESI + 1;
    puVar3 = puVar3 + 1;
  }
  for (iVar1 = 0x40; iVar1 != 0; iVar1 = iVar1 + -1) {
    *puVar3 = 0;
    puVar3 = puVar3 + 1;
  }
  puVar2 = auStack_630;
  for (iVar1 = 0x80; iVar1 != 0; iVar1 = iVar1 + -1) {
    *puVar3 = *puVar2;
    puVar2 = puVar2 + 1;
    puVar3 = puVar3 + 1;
  }
  uVar4 = FUN_100014d4(0,extraout_EDX);
  out((short)((ulonglong)uVar4 >> 0x20),(int)uVar4);
                    /* WARNING: Bad instruction - Truncating control flow here */
  halt_baddata();
}


// ==== FUN_100014d4 @ 100014d4 size=66

/* WARNING: Unable to track spacebase fully for stack */

undefined8 __fastcall FUN_100014d4(undefined4 param_1,undefined4 param_2)

{
  int iVar1;
  int iVar2;
  undefined4 in_EAX;
  int iVar3;
  undefined4 extraout_ECX;
  int extraout_ECX_00;
  undefined4 extraout_ECX_01;
  undefined4 extraout_ECX_02;
  int unaff_EBP;
  int *piVar4;
  undefined4 *unaff_EDI;
  int *piVar5;
  undefined8 uVar6;
  undefined4 *unaff_retaddr;
  
  for (iVar3 = CONCAT31((int3)((uint)param_1 >> 8),0x40); iVar3 != 0; iVar3 = iVar3 + -1) {
    *unaff_EDI = *unaff_retaddr;
    unaff_retaddr = unaff_retaddr + 1;
    unaff_EDI = unaff_EDI + 1;
  }
  for (iVar3 = 0x40; iVar3 != 0; iVar3 = iVar3 + -1) {
    *unaff_EDI = in_EAX;
    unaff_EDI = unaff_EDI + 1;
  }
  iVar3 = 0x10;
  do {
    uVar6 = FUN_10001284(iVar3,param_2);
    uVar6 = FUN_1000132c(extraout_ECX,(int)((ulonglong)uVar6 >> 0x20));
    param_2 = (undefined4)((ulonglong)uVar6 >> 0x20);
    iVar3 = extraout_ECX_00 + -1;
  } while (iVar3 != 0);
  uVar6 = FUN_10001284(0,param_2);
  FUN_1000132c(extraout_ECX_01,(int)((ulonglong)uVar6 >> 0x20));
  iVar3 = CONCAT31((int3)((uint)extraout_ECX_02 >> 8),4);
  piVar4 = (int *)(unaff_EBP + -0x10);
  piVar5 = (int *)&stack0x00000004;
  do {
    if (iVar3 == 0) break;
    iVar3 = iVar3 + -1;
    iVar2 = *piVar5;
    iVar1 = *piVar4;
    piVar4 = piVar4 + 1;
    piVar5 = piVar5 + 1;
  } while (iVar1 == iVar2);
  return CONCAT44(*(undefined4 *)(unaff_EBP + 0x14),*(undefined4 *)(unaff_EBP + 0x1c));
}


// ==== FUN_10001516 @ 10001516 size=236

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Removing unreachable block (ram,0x1000159f) */
/* WARNING: Removing unreachable block (ram,0x10001589) */
/* WARNING: Removing unreachable block (ram,0x1000159b) */
/* WARNING: Removing unreachable block (ram,0x100015a9) */
/* WARNING: Removing unreachable block (ram,0x100015fc) */

void FUN_10001516(void)

{
  byte bVar1;
  undefined2 uVar2;
  uint in_EAX;
  uint uVar3;
  int extraout_ECX;
  undefined2 extraout_DX;
  int iVar4;
  uint uVar5;
  uint uVar6;
  int *piVar7;
  uint uVar8;
  uint uVar9;
  uint uVar10;
  int *unaff_ESI;
  char *unaff_EDI;
  char *pcVar11;
  undefined8 uVar12;
  int iVar13;
  uint uVar14;
  uint uVar15;
  
  unaff_EDI[in_EAX] = -0x80;
  pcVar11 = unaff_EDI + in_EAX + 1;
  iVar4 = -(in_EAX + 1 & 0x3f);
  uVar5 = iVar4 + 0x40;
  uVar15 = uVar5;
  if (uVar5 < 8) {
    uVar5 = iVar4 + 0x80;
    uVar15 = uVar5;
  }
  for (; uVar5 != 0; uVar5 = uVar5 - 1) {
    *pcVar11 = '\0';
    pcVar11 = pcVar11 + 1;
  }
  uVar6 = uVar15 + 1 + in_EAX;
  *(ulonglong *)(pcVar11 + -8) = (ulonglong)in_EAX * 8;
  *unaff_ESI = 0x67452301;
  unaff_ESI[1] = -0x10325477;
  unaff_ESI[2] = -0x67452302;
  unaff_ESI[3] = 0x10325476;
  uVar15 = unaff_ESI[3];
  uVar5 = unaff_ESI[2];
  uVar14 = unaff_ESI[1];
  iVar4 = *unaff_ESI;
  iVar13 = 0;
  bVar1 = 0;
  piVar7 = (int *)&stack0xffffffcc;
  uVar3 = uVar15;
  uVar9 = uVar14;
  uVar10 = uVar5;
  while( true ) {
    uVar8 = uVar9;
    if (bVar1 < 0x30) {
      if (bVar1 < 0x20) {
        if (bVar1 < 0x10) {
          uVar10 = uVar10 & uVar8 | ~uVar8 & uVar3;
        }
        else {
          uVar9 = uVar8 & uVar3;
          uVar3 = ~uVar3;
          uVar10 = uVar10 & uVar3 | uVar9;
        }
      }
      else {
        uVar10 = uVar10 ^ uVar8 ^ uVar3;
      }
    }
    else {
      uVar3 = ~uVar3 | uVar8;
      uVar10 = uVar10 ^ uVar3;
    }
    bVar1 = 0xe7;
    uVar12 = FUN_10001767(uVar3,piVar7,iVar13,iVar4,uVar14,uVar5,uVar15,uVar6);
    *unaff_EDI = *unaff_EDI + (char)uVar12;
    if (-1 < *unaff_EDI) break;
    piVar7 = (int *)&stack0xffffffd8;
    uVar3 = uVar10;
    uVar9 = (uint)((ulonglong)uVar12 >> 0x20);
    uVar10 = uVar8;
  }
  uVar2 = func_0x80db26ff();
  (&stack0xffffffc0)[extraout_ECX] = (&stack0xffffffc0)[extraout_ECX] & (byte)((ushort)uVar2 >> 8);
  out(extraout_DX,(char)uVar2);
  swi(4);
                    /* WARNING: Bad instruction - Truncating control flow here */
  halt_baddata();
}


// ==== FUN_10001767 @ 10001767 size=69

/* WARNING: Control flow encountered bad instruction data */
/* WARNING: Removing unreachable block (ram,0x100015fc) */

undefined8 __fastcall
FUN_10001767(undefined4 param_1,int *param_2,int param_3,int param_4,int param_5,int param_6,
            int param_7,uint param_8)

{
  byte bVar1;
  undefined2 uVar2;
  int in_EAX;
  byte *pbVar3;
  uint uVar4;
  int extraout_ECX;
  undefined2 extraout_DX;
  int *piVar5;
  uint uVar6;
  uint uVar7;
  uint uVar8;
  int unaff_EBX;
  uint uVar9;
  int unaff_EBP;
  uint uVar10;
  int *unaff_ESI;
  char *unaff_EDI;
  undefined8 uVar11;
  uint uVar12;
  int unaff_retaddr;
  undefined4 in_stack_00000030;
  undefined4 in_stack_00000038;
  
  pbVar3 = (byte *)(in_EAX * 6 + unaff_retaddr);
  uVar10 = unaff_EBP + *param_2 + *(int *)(unaff_EDI + *pbVar3) + *(int *)(pbVar3 + 2);
  *param_2 = unaff_EBX + (uVar10 << (pbVar3[1] & 0x1f) | uVar10 >> 0x20 - (pbVar3[1] & 0x1f));
  uVar10 = param_3 + 1;
  if (0x3f < uVar10) {
    *unaff_ESI = *unaff_ESI + param_4;
    unaff_ESI[1] = unaff_ESI[1] + param_5;
    unaff_ESI[2] = unaff_ESI[2] + param_6;
    unaff_ESI[3] = unaff_ESI[3] + param_7;
    unaff_EDI = unaff_EDI + 0x40;
    if (param_8 < 0x40) {
      return CONCAT44(in_stack_00000030,in_stack_00000038);
    }
    uVar10 = 0;
    param_7 = unaff_ESI[3];
    param_6 = unaff_ESI[2];
    param_5 = unaff_ESI[1];
    param_4 = *unaff_ESI;
    param_8 = param_8 - 0x40;
  }
  piVar5 = &param_4;
  bVar1 = (byte)uVar10 & 3;
  uVar4 = param_7;
  uVar8 = param_5;
  uVar7 = param_6;
  uVar12 = uVar10;
  if ((uVar10 & 3) != 0) {
    uVar6 = param_4;
    uVar7 = param_5;
    uVar4 = param_6;
    if (bVar1 == 1) goto LAB_1000158f;
    if (bVar1 == 2) {
      piVar5 = &param_6;
      uVar4 = param_5;
      uVar8 = param_7;
      uVar7 = param_4;
    }
    else {
      piVar5 = &param_5;
      uVar4 = param_4;
      uVar8 = param_6;
      uVar7 = param_7;
    }
  }
  while( true ) {
    bVar1 = (byte)uVar12;
    if (bVar1 < 0x30) {
      if (bVar1 < 0x20) {
        if (bVar1 < 0x10) {
          uVar9 = uVar7 & uVar8 | ~uVar8 & uVar4;
        }
        else {
          uVar9 = uVar8 & uVar4;
          uVar4 = ~uVar4;
          uVar9 = uVar7 & uVar4 | uVar9;
        }
      }
      else {
        uVar9 = uVar7 ^ uVar8 ^ uVar4;
      }
    }
    else {
      uVar4 = ~uVar4 | uVar8;
      uVar9 = uVar7 ^ uVar4;
    }
    uVar12 = 0x100015e7;
    uVar11 = FUN_10001767(uVar4,piVar5,uVar10,param_4,param_5,param_6,param_7,param_8);
    uVar6 = (uint)((ulonglong)uVar11 >> 0x20);
    *unaff_EDI = *unaff_EDI + (char)uVar11;
    uVar7 = uVar8;
    uVar4 = uVar9;
    if (-1 < *unaff_EDI) break;
LAB_1000158f:
    piVar5 = &param_7;
    uVar8 = uVar6;
  }
  uVar2 = func_0x80db26ff();
  (&stack0xfffffffc)[extraout_ECX] = (&stack0xfffffffc)[extraout_ECX] & (byte)((ushort)uVar2 >> 8);
  out(extraout_DX,(char)uVar2);
  swi(4);
                    /* WARNING: Bad instruction - Truncating control flow here */
  halt_baddata();
}


// ==== FUN_100017ac @ 100017ac size=642

undefined8 __fastcall
FUN_100017ac(undefined4 param_1,undefined4 param_2,byte *param_3,int param_4,undefined4 *param_5,
            int *param_6)

{
  ushort *puVar1;
  uint uVar2;
  int iVar3;
  undefined4 uVar4;
  ushort uVar5;
  byte bVar6;
  undefined4 in_EAX;
  ushort *puVar7;
  undefined4 *puVar8;
  uint uVar9;
  int iVar10;
  uint uVar11;
  undefined4 *puVar12;
  undefined4 *puVar13;
  ushort *puVar14;
  ushort *puVar15;
  byte *pbVar16;
  undefined4 *puVar17;
  
  if (param_4 != 0) {
    bVar6 = 0xa5;
    iVar10 = param_4;
    pbVar16 = param_3;
    do {
      bVar6 = bVar6 * -3 - 0x3d;
      *pbVar16 = *pbVar16 ^ bVar6;
      pbVar16 = pbVar16 + 1;
      iVar10 = iVar10 + -1;
    } while (iVar10 != 0);
  }
  puVar7 = (ushort *)(param_3 + param_4 + -3);
  puVar8 = (undefined4 *)((int)param_5 + *param_6);
  puVar15 = (ushort *)(param_3 + 1);
  bVar6 = *param_3;
  puVar17 = param_5;
  if (bVar6 < 0x12) goto LAB_10001826;
  uVar9 = (uint)(byte)(bVar6 - 0xe);
LAB_10001831:
  if (((undefined4 *)((uVar9 - 3) + (int)puVar17) <= puVar8) &&
     ((ushort *)((uVar9 - 3) + (int)puVar15) <= puVar7)) {
    uVar11 = uVar9 >> 2;
    uVar9 = (uVar9 ^ 3) & 3;
    do {
      uVar4 = *(undefined4 *)puVar15;
      puVar15 = puVar15 + 2;
      *puVar17 = uVar4;
      puVar17 = puVar17 + 1;
      uVar11 = uVar11 - 1;
    } while (uVar11 != 0);
    pbVar16 = (byte *)((int)puVar15 - uVar9);
    puVar17 = (undefined4 *)((int)puVar17 - uVar9);
    puVar15 = (ushort *)(pbVar16 + 1);
    bVar6 = *pbVar16;
    uVar9 = (uint)bVar6;
    if (0xf < bVar6) goto LAB_1000189d;
    if ((undefined4 *)((int)puVar17 + 3U) <= puVar8) {
      puVar14 = (ushort *)(pbVar16 + 2);
      puVar12 = (undefined4 *)
                ((int)puVar17 + (-0x801 - ((uint)(bVar6 >> 2) + (uint)(byte)*puVar15 * 4)));
      if (param_5 <= puVar12) {
        *puVar17 = *puVar12;
        puVar12 = (undefined4 *)((int)puVar17 + 3);
LAB_10001931:
        uVar9 = (byte)puVar14[-1] & 3;
        puVar17 = puVar12;
        if (((byte)puVar14[-1] & 3) == 0) {
          if (puVar7 < puVar14) goto LAB_10001a15;
          puVar15 = (ushort *)((int)puVar14 + 1);
          bVar6 = (byte)*puVar14;
LAB_10001826:
          uVar9 = (uint)bVar6;
          if (bVar6 < 0x10) goto code_r0x1000182a;
        }
        else {
          if ((puVar8 < (undefined4 *)(uVar9 + (int)puVar12)) ||
             (puVar7 < (ushort *)(uVar9 + (int)puVar14))) goto LAB_10001a15;
          *puVar12 = *(undefined4 *)puVar14;
          puVar17 = (undefined4 *)((int)puVar12 + uVar9);
          puVar15 = (ushort *)((byte *)((int)puVar14 + uVar9) + 1);
          uVar9 = (uint)*(byte *)((int)puVar14 + uVar9);
        }
LAB_1000189d:
        bVar6 = (byte)uVar9;
        if (bVar6 < 0x40) {
          if (0x1f < bVar6) {
            iVar10 = 0;
            puVar14 = puVar15;
            if ((uVar9 & 0x1f) == 0) {
              while( true ) {
                puVar15 = (ushort *)((int)puVar14 + 1);
                if ((byte)*puVar14 != 0) break;
                iVar10 = iVar10 + 0xff;
                puVar1 = puVar14 + 2;
                puVar14 = puVar15;
                if (puVar7 < puVar1) goto LAB_10001a15;
              }
              uVar11 = (byte)*puVar14 + 0x24 + iVar10;
            }
            else {
              uVar11 = (uVar9 & 0x1f) + 5;
            }
            puVar14 = puVar15 + 1;
            puVar13 = (undefined4 *)((int)puVar17 + (-1 - (uint)(*puVar15 >> 2)));
            if (*puVar15 >> 2 < 3) goto LAB_10001963;
            goto LAB_10001906;
          }
          if (0xf < bVar6) {
            iVar10 = 0;
            puVar14 = puVar15;
            if ((uVar9 & 7) == 0) {
              while( true ) {
                puVar15 = (ushort *)((int)puVar14 + 1);
                if ((byte)*puVar14 != 0) break;
                iVar10 = iVar10 + 0xff;
                puVar1 = puVar14 + 2;
                puVar14 = puVar15;
                if (puVar7 < puVar1) goto LAB_10001a15;
              }
              uVar11 = iVar10 + 0xc + (uint)(byte)*puVar14;
            }
            else {
              uVar11 = (uVar9 & 7) + 5;
            }
            puVar14 = puVar15 + 1;
            uVar9 = CONCAT22((short)(((uVar9 & 8) << 0xd) >> 0x10),*puVar15) >> 2;
            if (uVar9 != 0) {
              puVar13 = (undefined4 *)((int)puVar17 + (-0x4000 - uVar9));
              goto LAB_10001906;
            }
            goto LAB_10001a15;
          }
          if (puVar8 < (undefined4 *)((int)puVar17 + 2U)) goto LAB_10001a15;
          puVar14 = (ushort *)((int)puVar15 + 1);
          puVar12 = (undefined4 *)((int)puVar17 + (-1 - ((uVar9 >> 2) + (uint)(byte)*puVar15 * 4)));
          if (puVar12 < param_5) goto LAB_10001a15;
          *(undefined1 *)puVar17 = *(undefined1 *)puVar12;
          *(undefined1 *)((int)puVar17 + 1) = *(undefined1 *)((int)puVar12 + 1);
          puVar12 = (undefined4 *)((int)puVar17 + 2);
        }
        else {
          puVar14 = (ushort *)((int)puVar15 + 1);
          uVar2 = (uVar9 >> 2 & 7) + (uint)(byte)*puVar15 * 8;
          puVar13 = (undefined4 *)((int)puVar17 + (-1 - uVar2));
          uVar11 = (uVar9 >> 5) + 4;
          if (uVar2 < 3) {
LAB_10001963:
            if ((puVar13 < param_5) || (puVar8 < (undefined4 *)((uVar11 - 3) + (int)puVar17)))
            goto LAB_10001a15;
            puVar12 = puVar17;
            for (iVar10 = uVar11 - 3; iVar10 != 0; iVar10 = iVar10 + -1) {
              *(undefined1 *)puVar12 = *(undefined1 *)puVar13;
              puVar13 = (undefined4 *)((int)puVar13 + 1);
              puVar12 = (undefined4 *)((int)puVar12 + 1);
            }
          }
          else {
LAB_10001906:
            if (puVar13 < param_5) goto LAB_10001a15;
            puVar12 = (undefined4 *)((uVar11 - 3) + (int)puVar17);
            uVar11 = uVar11 >> 2;
            if (puVar8 < puVar12) goto LAB_10001a15;
            do {
              uVar4 = *puVar13;
              puVar13 = puVar13 + 1;
              *puVar17 = uVar4;
              puVar17 = puVar17 + 1;
              uVar11 = uVar11 - 1;
            } while (uVar11 != 0);
          }
        }
        goto LAB_10001931;
      }
    }
  }
LAB_10001a15:
  *param_6 = (int)puVar17 - (int)param_5;
  return CONCAT44(param_2,in_EAX);
code_r0x1000182a:
  iVar10 = 0;
  if (bVar6 == 0) {
    while( true ) {
      uVar5 = *puVar15;
      puVar15 = (ushort *)((int)puVar15 + 1);
      if ((byte)uVar5 != 0) break;
      iVar3 = iVar10 + 0x111;
      iVar10 = iVar10 + 0xff;
      if (puVar7 < (ushort *)(iVar3 + (int)puVar15)) goto LAB_10001a15;
    }
    uVar9 = (byte)uVar5 + 0x15 + iVar10;
  }
  else {
    uVar9 = uVar9 + 6;
  }
  goto LAB_10001831;
}


// ==== GetPluginAPI @ 10001a2e size=26

void __cdecl GetPluginAPI(undefined4 *param_1)

{
                    /* 0x1a2e  4  GetPluginAPI */
  *param_1 = 4;
  param_1[1] = 0xffffffff;
  param_1[2] = 0;
  param_1[3] = 0;
  return;
}


// ==== OpenConfig @ 10001a48 size=3

undefined1 OpenConfig(void)

{
                    /* 0x1a48  5  OpenConfig */
  return 1;
}


// ==== GetData @ 10001c21 size=69

undefined8 __fastcall GetData(undefined4 param_1,undefined4 param_2)

{
                    /* 0x1c21  3  GetData */
  FUN_10001c66();
  FUN_10001c66();
  FUN_10002666();
  return CONCAT44(&stack0x00000000,param_2);
}


// ==== FUN_10001c66 @ 10001c66 size=26

void FUN_10001c66(void)

{
  int unaff_EBX;
  
  FUN_10002666();
  if (unaff_EBX != 0) {
    FUN_10002660();
    FUN_10002660();
  }
  return;
}


// ==== FUN_10001cb9 @ 10001cb9 size=288

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

longlong FUN_10001cb9(int param_1,uint param_2)

{
  int iVar1;
  int iVar2;
  int iVar3;
  uint uVar4;
  uint in_stack_ffffffac;
  
  iVar2 = FUN_10001165();
  if (iVar2 != 0) {
    _DAT_10001fa4 = iVar2;
    iVar3 = FUN_10002642();
    iVar1 = DAT_100027da;
    if ((iVar3 != 0) && (param_1 == 0x4e4)) {
      in_stack_ffffffac = 0x10001d30;
      iVar3 = FUN_10002642();
      if ((iVar3 != 0) && (param_1 == DAT_100027da)) {
        in_stack_ffffffac = 0;
        uVar4 = FUN_100011b6(0x4e4,param_2,iVar2,0,0);
        if (uVar4 != 0) {
          in_stack_ffffffac = uVar4;
          iVar3 = FUN_1000263c();
          if (iVar3 == 0) {
            iVar3 = FUN_1000261e();
            FUN_10002618();
            if ((uVar4 == 0) || (iVar3 == 0)) {
              return CONCAT44(iVar2 + 0x4e4,CONCAT31((int3)((uint)iVar1 >> 8),1));
            }
            FUN_1000266c();
            FUN_10001194();
          }
          else {
            FUN_10002618();
            FUN_10001194();
          }
        }
      }
    }
  }
  FUN_10002672();
  return (ulonglong)in_stack_ffffffac << 0x20;
}


// ==== ApplyPatchSuspended @ 10001ddb size=29

longlong __fastcall
ApplyPatchSuspended(undefined4 param_1,undefined4 param_2,int param_3,uint param_4)

{
  undefined4 in_EAX;
  longlong lVar1;
  
                    /* 0x1ddb  2  ApplyPatchSuspended */
  if (DAT_10001fb4 != '\0') {
    lVar1 = FUN_10001cb9(param_3,param_4);
    return lVar1;
  }
  return CONCAT44(param_2,CONCAT31((int3)((uint)in_EAX >> 8),1));
}


// ==== ApplyPatch @ 10001df8 size=29

longlong __fastcall ApplyPatch(undefined4 param_1,undefined4 param_2,int param_3,uint param_4)

{
  undefined4 in_EAX;
  longlong lVar1;
  
                    /* 0x1df8  1  ApplyPatch */
  if (DAT_10001fb4 == '\0') {
    lVar1 = FUN_10001cb9(param_3,param_4);
    return lVar1;
  }
  return CONCAT44(param_2,CONCAT31((int3)((uint)in_EAX >> 8),1));
}


// ==== FUN_100025dd @ 100025dd size=29

void FUN_100025dd(void)

{
  if (DAT_10001c0c != 0) {
    FUN_10002636();
  }
  if (DAT_10001c10 != 0) {
    FUN_10002636();
  }
  return;
}


// ==== FUN_100025fa @ 100025fa size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_100025fa(void)

{
                    /* WARNING: Could not recover jumptable at 0x100025fa. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001000)();
  return;
}


// ==== FUN_10002600 @ 10002600 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002600(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002600. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*ram0x10001004)();
  return;
}


// ==== FUN_10002606 @ 10002606 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002606(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002606. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001008)();
  return;
}


// ==== FUN_1000260c @ 1000260c size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_1000260c(void)

{
                    /* WARNING: Could not recover jumptable at 0x1000260c. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_1000100c)();
  return;
}


// ==== FUN_10002612 @ 10002612 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002612(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002612. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001010)();
  return;
}


// ==== FUN_10002618 @ 10002618 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002618(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002618. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001014)();
  return;
}


// ==== FUN_1000261e @ 1000261e size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_1000261e(void)

{
                    /* WARNING: Could not recover jumptable at 0x1000261e. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001018)();
  return;
}


// ==== FUN_10002624 @ 10002624 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002624(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002624. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_1000101c)();
  return;
}


// ==== FUN_1000262a @ 1000262a size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_1000262a(void)

{
                    /* WARNING: Could not recover jumptable at 0x1000262a. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001020)();
  return;
}


// ==== FUN_10002630 @ 10002630 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002630(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002630. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001024)();
  return;
}


// ==== FUN_10002636 @ 10002636 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002636(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002636. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001028)();
  return;
}


// ==== FUN_1000263c @ 1000263c size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_1000263c(void)

{
                    /* WARNING: Could not recover jumptable at 0x1000263c. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_1000102c)();
  return;
}


// ==== FUN_10002642 @ 10002642 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002642(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002642. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001030)();
  return;
}


// ==== FUN_10002648 @ 10002648 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002648(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002648. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001034)();
  return;
}


// ==== FUN_1000264e @ 1000264e size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_1000264e(void)

{
                    /* WARNING: Could not recover jumptable at 0x1000264e. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001038)();
  return;
}


// ==== FUN_10002654 @ 10002654 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002654(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002654. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_1000103c)();
  return;
}


// ==== FUN_1000265a @ 1000265a size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_1000265a(void)

{
                    /* WARNING: Could not recover jumptable at 0x1000265a. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001040)();
  return;
}


// ==== FUN_10002660 @ 10002660 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002660(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002660. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001044)();
  return;
}


// ==== FUN_10002666 @ 10002666 size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_10002666(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002666. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001048)();
  return;
}


// ==== FUN_1000266c @ 1000266c size=6

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

void FUN_1000266c(void)

{
                    /* WARNING: Could not recover jumptable at 0x1000266c. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*_DAT_10001054)();
  return;
}


// ==== FUN_10002672 @ 10002672 size=6

void FUN_10002672(void)

{
                    /* WARNING: Could not recover jumptable at 0x10002672. Too many branches */
                    /* WARNING: Treating indirect jump as call */
  (*DAT_10001050)();
  return;
}


// ==== entry @ 100062e0 size=1176

/* WARNING: Globals starting with '_' overlap smaller symbols at the same address */

longlong __fastcall entry(undefined4 param_1,uint param_2,char *param_3,int param_4)

{
  undefined1 uVar1;
  byte bVar2;
  code *pcVar3;
  char *pcVar4;
  int iVar5;
  uint uVar6;
  undefined4 uVar7;
  undefined4 extraout_ECX;
  int iVar8;
  int *piVar9;
  undefined4 *puVar10;
  uint uVar11;
  uint uVar12;
  int *piVar13;
  uint uVar14;
  char *pcVar15;
  uint *puVar16;
  undefined4 *puVar17;
  int *piVar18;
  int *piVar19;
  int *piVar20;
  byte *pbVar21;
  byte *pbVar22;
  bool bVar23;
  bool bVar24;
  bool bVar25;
  undefined8 uVar26;
  undefined1 auStackY_fc [76];
  undefined4 uStackY_b0;
  uint uStack_88;
  int aiStack_7c [2];
  int iStack_74;
  int iStack_6c;
  undefined4 uStack_68;
  undefined4 *puStack_60;
  undefined4 uStack_5c;
  int iStack_54;
  undefined4 uStack_50;
  undefined2 *puStack_48;
  char *pcStack_44;
  undefined4 uStack_40;
  char *pcStack_3c;
  char *pcStack_38;
  char *pcStack_34;
  int iStack_30;
  char *pcStack_2c;
  char *pcStack_28;
  int *piStack_24;
  
  if ((char)param_4 == '\x01') {
    puVar16 = &DAT_10005000;
    puVar17 = (undefined4 *)&DAT_10001000;
    uVar14 = 0xffffffff;
    do {
      uVar11 = *puVar16;
      bVar23 = puVar16 < (uint *)0xfffffffc;
      puVar16 = puVar16 + 1;
      bVar24 = CARRY4(uVar11,uVar11) || CARRY4(uVar11 * 2,(uint)bVar23);
      uVar11 = uVar11 * 2 + (uint)bVar23;
      do {
        if (bVar24) {
          uVar6 = *puVar16;
          puVar16 = (uint *)((int)puVar16 + 1);
          *(char *)puVar17 = (char)uVar6;
          puVar17 = (undefined4 *)((int)puVar17 + 1);
        }
        else {
          uVar6 = 1;
          do {
            do {
              bVar23 = CARRY4(uVar11,uVar11);
              uVar12 = uVar11 * 2;
              if (uVar12 == 0) {
                uVar11 = *puVar16;
                bVar24 = puVar16 < (uint *)0xfffffffc;
                puVar16 = puVar16 + 1;
                bVar23 = CARRY4(uVar11,uVar11) || CARRY4(uVar11 * 2,(uint)bVar24);
                uVar12 = uVar11 * 2 + (uint)bVar24;
              }
              uVar6 = uVar6 * 2 + (uint)bVar23;
              uVar11 = uVar12 * 2;
            } while (!CARRY4(uVar12,uVar12));
            if (uVar11 != 0) break;
            uVar12 = *puVar16;
            bVar23 = puVar16 < (uint *)0xfffffffc;
            puVar16 = puVar16 + 1;
            uVar11 = uVar12 * 2 + (uint)bVar23;
          } while (!CARRY4(uVar12,uVar12) && !CARRY4(uVar12 * 2,(uint)bVar23));
          if (2 < uVar6) {
            uVar14 = *puVar16;
            puVar16 = (uint *)((int)puVar16 + 1);
            uVar14 = CONCAT31((int3)uVar6 + -3,(char)uVar14) ^ 0xffffffff;
            if (uVar14 == 0) {
              pcVar15 = &DAT_10001000;
              iVar8 = 0x141d;
              bVar23 = true;
              goto LAB_100063c4;
            }
          }
          bVar23 = CARRY4(uVar11,uVar11);
          uVar11 = uVar11 * 2;
          if (uVar11 == 0) {
            uVar11 = *puVar16;
            bVar24 = puVar16 < (uint *)0xfffffffc;
            puVar16 = puVar16 + 1;
            bVar23 = CARRY4(uVar11,uVar11) || CARRY4(uVar11 * 2,(uint)bVar24);
            uVar11 = uVar11 * 2 + (uint)bVar24;
          }
          bVar24 = CARRY4(uVar11,uVar11);
          uVar11 = uVar11 * 2;
          if (uVar11 == 0) {
            uVar11 = *puVar16;
            bVar25 = puVar16 < (uint *)0xfffffffc;
            puVar16 = puVar16 + 1;
            bVar24 = CARRY4(uVar11,uVar11) || CARRY4(uVar11 * 2,(uint)bVar25);
            uVar11 = uVar11 * 2 + (uint)bVar25;
          }
          iVar8 = (uint)bVar23 * 2 + (uint)bVar24;
          if (iVar8 == 0) {
            iVar8 = 1;
            do {
              do {
                bVar23 = CARRY4(uVar11,uVar11);
                uVar6 = uVar11 * 2;
                if (uVar6 == 0) {
                  uVar11 = *puVar16;
                  bVar24 = puVar16 < (uint *)0xfffffffc;
                  puVar16 = puVar16 + 1;
                  bVar23 = CARRY4(uVar11,uVar11) || CARRY4(uVar11 * 2,(uint)bVar24);
                  uVar6 = uVar11 * 2 + (uint)bVar24;
                }
                iVar8 = iVar8 * 2 + (uint)bVar23;
                uVar11 = uVar6 * 2;
              } while (!CARRY4(uVar6,uVar6));
              if (uVar11 != 0) break;
              uVar6 = *puVar16;
              bVar23 = puVar16 < (uint *)0xfffffffc;
              puVar16 = puVar16 + 1;
              uVar11 = uVar6 * 2 + (uint)bVar23;
            } while (!CARRY4(uVar6,uVar6) && !CARRY4(uVar6 * 2,(uint)bVar23));
            iVar8 = iVar8 + 2;
          }
          uVar6 = iVar8 + 1 + (uint)(uVar14 < 0xfffff300);
          puVar10 = (undefined4 *)((int)puVar17 + uVar14);
          if (uVar14 < 0xfffffffd) {
            do {
              uVar7 = *puVar10;
              puVar10 = puVar10 + 1;
              *puVar17 = uVar7;
              puVar17 = puVar17 + 1;
              bVar23 = 3 < uVar6;
              uVar6 = uVar6 - 4;
            } while (bVar23 && uVar6 != 0);
            puVar17 = (undefined4 *)((int)puVar17 + uVar6);
          }
          else {
            do {
              uVar1 = *(undefined1 *)puVar10;
              puVar10 = (undefined4 *)((int)puVar10 + 1);
              *(undefined1 *)puVar17 = uVar1;
              puVar17 = (undefined4 *)((int)puVar17 + 1);
              uVar6 = uVar6 - 1;
            } while (uVar6 != 0);
          }
        }
        bVar24 = CARRY4(uVar11,uVar11);
        uVar11 = uVar11 * 2;
      } while (uVar11 != 0);
    } while( true );
  }
  goto LAB_100022f9;
LAB_100063c4:
  pcVar4 = pcVar15;
  if (iVar8 != 0) goto LAB_100063c4;
  goto code_r0x100063c6;
LAB_100063c4:
  iVar8 = iVar8 + -1;
  pcVar4 = pcVar15 + 1;
  bVar23 = *pcVar15 == -0x18;
  pcVar15 = pcVar4;
  if (!bVar23) goto LAB_100063c4;
code_r0x100063c6:
  if (bVar23) {
    bVar23 = false;
    pcVar15 = pcVar4;
    if (*pcVar4 == '\x01') {
      uVar7 = *(undefined4 *)pcVar4;
      bVar23 = &DAT_10001000 +
               (CONCAT31(CONCAT21((ushort)uVar7 >> 8,(char)((uint)uVar7 >> 0x10)),
                         (char)((uint)uVar7 >> 0x18)) - (int)pcVar4) == (undefined1 *)0x0;
      pcVar15 = pcVar4 + 4;
      *(undefined1 **)pcVar4 =
           &DAT_10001000 +
           (CONCAT31(CONCAT21((ushort)uVar7 >> 8,(char)((uint)uVar7 >> 0x10)),
                     (char)((uint)uVar7 >> 0x18)) - (int)pcVar4);
    }
    goto LAB_100063c4;
  }
  piVar18 = &DAT_10005000;
  while (*piVar18 != 0) {
    piStack_24 = (int *)((int)&DWORD_100073d0 + *piVar18);
    piVar13 = (int *)(&DAT_10001000 + piVar18[1]);
    piVar20 = piVar18 + 2;
    pcStack_28 = (char *)0x10006401;
    uVar7 = (*(code *)PTR_DAT_1000740c)();
    while( true ) {
      iVar8 = *piVar20;
      piVar18 = (int *)((int)piVar20 + 1);
      if ((char)iVar8 == '\0') break;
      piVar9 = piVar18;
      piVar19 = piVar18;
      do {
        piVar20 = piVar19;
        if (piVar9 == (int *)0x0) break;
        piVar9 = (int *)((int)piVar9 + -1);
        piVar20 = (int *)((int)piVar19 + 1);
        iVar5 = *piVar19;
        piVar19 = piVar20;
      } while ((char)((char)iVar8 + -1) != (char)iVar5);
      pcStack_2c = (char *)0x10006416;
      pcStack_28 = (char *)uVar7;
      piStack_24 = piVar18;
      iVar8 = (*DAT_10007410)();
      if (iVar8 == 0) {
        return (ulonglong)param_2 << 0x20;
      }
      *piVar13 = iVar8;
      piVar13 = piVar13 + 1;
    }
  }
  puVar17 = (undefined4 *)0x10000ffc;
  pbVar21 = (byte *)(piVar18 + 1);
  while( true ) {
    pcVar3 = DAT_10007414;
    bVar2 = *pbVar21;
    piStack_24 = (int *)(uint)bVar2;
    pbVar22 = pbVar21 + 1;
    if (piStack_24 == (int *)0x0) break;
    if (0xef < bVar2) {
      piStack_24 = (int *)(CONCAT12(bVar2,*(undefined2 *)pbVar22) & 0xff0fffff);
      pbVar22 = pbVar21 + 3;
    }
    puVar17 = (undefined4 *)((int)puVar17 + (int)piStack_24);
    uVar7 = *puVar17;
    *puVar17 = &DAT_10001000 +
               CONCAT31(CONCAT21(CONCAT11((char)uVar7,(char)((uint)uVar7 >> 8)),
                                 (char)((uint)uVar7 >> 0x10)),(char)((uint)uVar7 >> 0x18));
    pbVar21 = pbVar22;
  }
  pcStack_28 = (char *)&piStack_24;
  pcStack_2c = (char *)0x4;
  iStack_30 = 0x1000;
  pcStack_38 = (char *)0x10006471;
  pcStack_34 = "MZh";
  (*DAT_10007414)();
  DAT_100001ff = DAT_100001ff & 0x7f;
  DAT_10000227 = DAT_10000227 & 0x7f;
  pcStack_3c = pcStack_34;
  uStack_40 = 0x1000;
  puStack_48 = (undefined2 *)0x10006486;
  pcStack_44 = "MZh";
  pcStack_38 = (char *)&pcStack_34;
  (*pcVar3)();
  do {
    aiStack_7c[0] = 0;
    param_2 = uStack_88;
  } while (aiStack_7c != (int *)auStackY_fc);
LAB_100022f9:
  if (param_4 != 1) {
    if (param_4 == 0) {
      pcStack_28 = (char *)0x100025d5;
      FUN_100025dd();
    }
    return CONCAT44(param_2,1);
  }
  pcStack_28 = &DAT_100029e2;
  pcStack_2c = (char *)0x10002311;
  FUN_1000262a();
  _DAT_10001e5a = DAT_100029e6;
  pcStack_2c = (char *)0x10002320;
  iVar8 = FUN_100010b0();
  pcVar15 = s_Error_initializing_remote_librar_10001bb6;
  if (iVar8 != 0) {
    pcStack_2c = s_kernel32_10002090;
    iStack_30 = 0x10002337;
    pcVar4 = (char *)FUN_10002606();
    pcVar15 = s_GMH_error__10001aeb;
    if (pcVar4 != (char *)0x0) {
      pcVar15 = s_GPA_error__10001af6;
      for (piVar18 = &DAT_10001aab; *piVar18 != 0; piVar18 = piVar18 + 2) {
        iStack_30 = *piVar18;
        pcStack_38 = (undefined1 *)0x10002359;
        pcStack_2c = pcVar4;
        iVar8 = FUN_1000260c();
        if (iVar8 == 0) goto LAB_100025ad;
        *(int *)piVar18[1] = iVar8;
      }
      iStack_30 = 0x104;
      pcStack_34 = s_9_wC__Users_Asus_AppData_Local_T_10002689 + 3;
      pcStack_2c = param_3;
      pcStack_3c = (char *)0x10002380;
      iVar8 = FUN_10002624();
      pcVar15 = s_GMFN_error__10001b01;
      if (iVar8 != 0) {
        pcStack_3c = (char *)0x20;
        pcStack_2c = s_9_wC__Users_Asus_AppData_Local_T_10002689 + 3;
        pcStack_44 = (char *)0x10002399;
        pcVar4 = (char *)FUN_10002654();
        pcVar15 = s_Error_opening_plugin_data_file__10001b0d;
        if (pcVar4 != (char *)0x0) {
          pcStack_44 = (char *)0x40;
          puStack_48 = &DAT_10002790;
          uStack_50 = 0x100023b4;
          pcStack_28 = pcVar4;
          iVar5 = FUN_1000265a();
          iVar8 = DAT_100027c8;
          pcVar15 = s_Error_reading_plugin_data_file__10001b2d;
          if (((iVar5 == 0x40) && (pcVar15 = s_Plugin_is_invalid__10001b4d, DAT_10002790 == 0x5a4d))
             && (DAT_100027c8 != 0)) {
            pcVar15 = s_Error_reading_plugin_data_file__10001b2d;
            uStack_50 = 0;
            iStack_54 = DAT_100027c8;
            uStack_5c = 0x100023f2;
            pcStack_28 = pcVar4;
            iVar5 = FUN_1000264e();
            if (iVar5 == iVar8) {
              uStack_5c = 0x12;
              puStack_60 = &DAT_100027d0;
              uStack_68 = 0x10002407;
              pcStack_28 = pcVar4;
              iVar5 = FUN_1000265a();
              if (iVar5 == 0x12) {
                uStack_68 = 0;
                iStack_6c = iVar8;
                iStack_74 = 0x10002419;
                pcStack_28 = pcVar4;
                iVar5 = FUN_1000264e();
                if ((iVar5 == iVar8) && (pcVar15 = s_Plugin_is_invalid__10001b4d, DAT_100027d6 != 0)
                   ) {
                  iVar5 = DAT_100027d6 + DAT_100027de;
                  iStack_74 = iVar5 + 0x52;
                  pcStack_28 = (char *)0x0;
                  aiStack_7c[0] = 0x10002445;
                  iVar8 = FUN_10002630();
                  pcVar15 = s_Out_of_memory__10001b60;
                  if (iVar8 != 0) {
                    pcVar15 = s_Error_reading_plugin_data_file__10001b2d;
                    iVar5 = iVar5 + 0x12;
                    DAT_10001c0c = iVar8;
                    aiStack_7c[0] = iVar5;
                    pcStack_28 = pcVar4;
                    iVar8 = FUN_1000265a();
                    if ((iVar8 == iVar5) &&
                       (pcStack_28 = pcVar4, iVar8 = FUN_1000265a(), iVar8 == 0x100)) {
                      pcStack_2c = pcVar4;
                      FUN_10002648();
                      pcVar15 = s_Plugin_is_invalid__10001b4d;
                      if ((DAT_100027d0 == 0x444f4d58) && (DAT_100027d4 == '\0')) {
                        bVar23 = (DAT_100027d5 & 3) == 0;
                        if ((DAT_100027d5 & 3) == 1) {
                          DAT_10001fb4 = 0;
                        }
                        FUN_100013a9();
                        pcVar15 = s_Plugin_is_damaged__Please_downlo_10001b6f;
                        if (!bVar23) {
                          pcStack_2c = (char *)0x0;
                          uVar26 = FUN_10002630();
                          puVar17 = (undefined4 *)uVar26;
                          pcVar15 = s_Out_of_memory__10001b60;
                          if (puVar17 != (undefined4 *)0x0) {
                            bVar23 = CARRY4(DAT_10001c0c + 0x12U,DAT_100027de);
                            uStackY_b0 = 0x1000251f;
                            DAT_10001c10 = puVar17;
                            FUN_100017ac(extraout_ECX,(int)((ulonglong)uVar26 >> 0x20),
                                         (byte *)(DAT_10001c0c + 0x12U + DAT_100027de),DAT_100027d6,
                                         puVar17,&DAT_100027da);
                            pcVar15 = s_Unknown_error__Sorry__10001ba0;
                            if (!bVar23) {
                              piVar18 = (int *)(DAT_10001c0c + 0x12);
                              uVar14 = DAT_100027de;
                              while( true ) {
                                iVar8 = *piVar18;
                                *(undefined1 *)piVar18 = 0;
                                if (((int)uVar14 < 9) || (iVar5 = piVar18[1], iVar5 < 0)) break;
                                piVar18 = piVar18 + 2;
                                piVar20 = piVar18;
                                piVar13 = DAT_10001bfc;
                                piVar9 = DAT_10001c00;
                                piVar19 = DAT_10001c08;
                                if ((((iVar8 != 0x454d414e) &&
                                     (piVar20 = DAT_10001bf8, piVar13 = piVar18, iVar8 != 0x43534544
                                     )) && (piVar13 = DAT_10001bfc, piVar9 = piVar18,
                                           iVar8 != 0x204c5255)) &&
                                   ((piVar9 = DAT_10001c00, piVar19 = piVar18, iVar8 != 0x20524556
                                    && (piVar19 = DAT_10001c08, iVar8 == 0x48545541)))) {
                                  DAT_10001c04 = piVar18;
                                }
                                DAT_10001c08 = piVar19;
                                DAT_10001c00 = piVar9;
                                DAT_10001bfc = piVar13;
                                DAT_10001bf8 = piVar20;
                                uVar14 = (uVar14 - 8) - iVar5;
                                piVar18 = (int *)((int)piVar18 + iVar5);
                              }
                              return 0x10000000001;
                            }
                          }
                        }
                      }
                      goto LAB_100025ad;
                    }
                  }
                }
              }
            }
          }
          iStack_30 = 0x100025ad;
          pcStack_2c = pcVar4;
          FUN_10002648();
        }
      }
    }
  }
LAB_100025ad:
  iStack_30 = 0x100025b2;
  FUN_100025dd();
  iStack_30 = 0x30;
  pcStack_34 = s_XeN_z_XMOD_loader_stub_for_BWL_10001bd9;
  pcStack_3c = (char *)0x0;
  uStack_40 = 0x100025c1;
  pcStack_38 = pcVar15;
  FUN_10002672();
  return ZEXT48(pcStack_28) << 0x20;
}


