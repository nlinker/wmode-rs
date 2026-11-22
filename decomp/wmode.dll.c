// ==== entry @ 100062e0 size=438

/* WARNING: Control flow encountered bad instruction data */

longlong __fastcall entry(undefined4 param_1,uint param_2,undefined4 param_3,char param_4)

{
  undefined1 uVar1;
  byte bVar2;
  undefined4 uVar3;
  int iVar4;
  uint uVar5;
  HMODULE hModule;
  FARPROC pFVar6;
  int iVar7;
  int *piVar8;
  undefined4 *puVar9;
  uint uVar10;
  uint uVar11;
  uint uVar12;
  uint *puVar13;
  undefined4 *puVar14;
  char *pcVar15;
  char *pcVar16;
  int *lpProcName;
  int *piVar17;
  int *piVar18;
  byte *pbVar19;
  byte *pbVar20;
  bool bVar21;
  bool bVar22;
  bool bVar23;
  undefined1 local_80 [72];
  undefined4 uStackY_38;
  
  if (param_4 != '\x01') {
    halt_baddata();
  }
  puVar13 = &DAT_10005000;
  puVar14 = (undefined4 *)&DAT_10001000;
  uVar12 = 0xffffffff;
  do {
    uVar10 = *puVar13;
    bVar21 = puVar13 < (uint *)0xfffffffc;
    puVar13 = puVar13 + 1;
    bVar22 = CARRY4(uVar10,uVar10) || CARRY4(uVar10 * 2,(uint)bVar21);
    uVar10 = uVar10 * 2 + (uint)bVar21;
    do {
      if (bVar22) {
        uVar5 = *puVar13;
        puVar13 = (uint *)((int)puVar13 + 1);
        *(char *)puVar14 = (char)uVar5;
        puVar14 = (undefined4 *)((int)puVar14 + 1);
      }
      else {
        uVar5 = 1;
        do {
          do {
            bVar21 = CARRY4(uVar10,uVar10);
            uVar11 = uVar10 * 2;
            if (uVar11 == 0) {
              uVar10 = *puVar13;
              bVar22 = puVar13 < (uint *)0xfffffffc;
              puVar13 = puVar13 + 1;
              bVar21 = CARRY4(uVar10,uVar10) || CARRY4(uVar10 * 2,(uint)bVar22);
              uVar11 = uVar10 * 2 + (uint)bVar22;
            }
            uVar5 = uVar5 * 2 + (uint)bVar21;
            uVar10 = uVar11 * 2;
          } while (!CARRY4(uVar11,uVar11));
          if (uVar10 != 0) break;
          uVar11 = *puVar13;
          bVar21 = puVar13 < (uint *)0xfffffffc;
          puVar13 = puVar13 + 1;
          uVar10 = uVar11 * 2 + (uint)bVar21;
        } while (!CARRY4(uVar11,uVar11) && !CARRY4(uVar11 * 2,(uint)bVar21));
        if (2 < uVar5) {
          uVar12 = *puVar13;
          puVar13 = (uint *)((int)puVar13 + 1);
          uVar12 = CONCAT31((int3)uVar5 + -3,(char)uVar12) ^ 0xffffffff;
          if (uVar12 == 0) {
            pcVar15 = &DAT_10001000;
            iVar7 = 0x141d;
            bVar21 = true;
            goto LAB_100063c4;
          }
        }
        bVar21 = CARRY4(uVar10,uVar10);
        uVar10 = uVar10 * 2;
        if (uVar10 == 0) {
          uVar10 = *puVar13;
          bVar22 = puVar13 < (uint *)0xfffffffc;
          puVar13 = puVar13 + 1;
          bVar21 = CARRY4(uVar10,uVar10) || CARRY4(uVar10 * 2,(uint)bVar22);
          uVar10 = uVar10 * 2 + (uint)bVar22;
        }
        bVar22 = CARRY4(uVar10,uVar10);
        uVar10 = uVar10 * 2;
        if (uVar10 == 0) {
          uVar10 = *puVar13;
          bVar23 = puVar13 < (uint *)0xfffffffc;
          puVar13 = puVar13 + 1;
          bVar22 = CARRY4(uVar10,uVar10) || CARRY4(uVar10 * 2,(uint)bVar23);
          uVar10 = uVar10 * 2 + (uint)bVar23;
        }
        iVar7 = (uint)bVar21 * 2 + (uint)bVar22;
        if (iVar7 == 0) {
          iVar7 = 1;
          do {
            do {
              bVar21 = CARRY4(uVar10,uVar10);
              uVar5 = uVar10 * 2;
              if (uVar5 == 0) {
                uVar10 = *puVar13;
                bVar22 = puVar13 < (uint *)0xfffffffc;
                puVar13 = puVar13 + 1;
                bVar21 = CARRY4(uVar10,uVar10) || CARRY4(uVar10 * 2,(uint)bVar22);
                uVar5 = uVar10 * 2 + (uint)bVar22;
              }
              iVar7 = iVar7 * 2 + (uint)bVar21;
              uVar10 = uVar5 * 2;
            } while (!CARRY4(uVar5,uVar5));
            if (uVar10 != 0) break;
            uVar5 = *puVar13;
            bVar21 = puVar13 < (uint *)0xfffffffc;
            puVar13 = puVar13 + 1;
            uVar10 = uVar5 * 2 + (uint)bVar21;
          } while (!CARRY4(uVar5,uVar5) && !CARRY4(uVar5 * 2,(uint)bVar21));
          iVar7 = iVar7 + 2;
        }
        uVar5 = iVar7 + 1 + (uint)(uVar12 < 0xfffff300);
        puVar9 = (undefined4 *)((int)puVar14 + uVar12);
        if (uVar12 < 0xfffffffd) {
          do {
            uVar3 = *puVar9;
            puVar9 = puVar9 + 1;
            *puVar14 = uVar3;
            puVar14 = puVar14 + 1;
            bVar21 = 3 < uVar5;
            uVar5 = uVar5 - 4;
          } while (bVar21 && uVar5 != 0);
          puVar14 = (undefined4 *)((int)puVar14 + uVar5);
        }
        else {
          do {
            uVar1 = *(undefined1 *)puVar9;
            puVar9 = (undefined4 *)((int)puVar9 + 1);
            *(undefined1 *)puVar14 = uVar1;
            puVar14 = (undefined4 *)((int)puVar14 + 1);
            uVar5 = uVar5 - 1;
          } while (uVar5 != 0);
        }
      }
      bVar22 = CARRY4(uVar10,uVar10);
      uVar10 = uVar10 * 2;
    } while (uVar10 != 0);
  } while( true );
LAB_100063c4:
  pcVar16 = pcVar15;
  if (iVar7 != 0) goto LAB_100063c4;
  goto code_r0x100063c6;
LAB_100063c4:
  iVar7 = iVar7 + -1;
  pcVar16 = pcVar15 + 1;
  bVar21 = *pcVar15 == -0x18;
  pcVar15 = pcVar16;
  if (bVar21) {
code_r0x100063c6:
    if (!bVar21) {
      lpProcName = &DAT_10005000;
      do {
        if (*lpProcName == 0) {
          puVar14 = (undefined4 *)0x10000ffc;
          pbVar19 = (byte *)(lpProcName + 1);
          while( true ) {
            bVar2 = *pbVar19;
            uVar12 = (uint)bVar2;
            pbVar20 = pbVar19 + 1;
            if (uVar12 == 0) break;
            if (0xef < bVar2) {
              uVar12 = CONCAT12(bVar2,*(undefined2 *)pbVar20) & 0xff0fffff;
              pbVar20 = pbVar19 + 3;
            }
            puVar14 = (undefined4 *)((int)puVar14 + uVar12);
            uVar3 = *puVar14;
            *puVar14 = &DAT_10001000 +
                       CONCAT31(CONCAT21(CONCAT11((char)uVar3,(char)((uint)uVar3 >> 8)),
                                         (char)((uint)uVar3 >> 0x10)),(char)((uint)uVar3 >> 0x18));
            pbVar19 = pbVar20;
          }
          uStackY_38 = 0x10006471;
          VirtualProtect(&IMAGE_DOS_HEADER_10000000,0x1000,4,(PDWORD)&stack0xffffffdc);
          DAT_100001ff = DAT_100001ff & 0x7f;
          DAT_10000227 = DAT_10000227 & 0x7f;
          uStackY_38 = 0x10006486;
          VirtualProtect(&IMAGE_DOS_HEADER_10000000,0x1000,uVar12,(PDWORD)&stack0xffffffdc);
          do {
          } while (&stack0x00000000 != local_80);
                    /* WARNING: Bad instruction - Truncating control flow here */
          halt_baddata();
        }
        puVar14 = (undefined4 *)(&DAT_10001000 + lpProcName[1]);
        piVar18 = lpProcName + 2;
        hModule = LoadLibraryA((LPCSTR)((int)&DWORD_100073d0 + *lpProcName));
        while( true ) {
          iVar7 = *piVar18;
          lpProcName = (int *)((int)piVar18 + 1);
          if ((char)iVar7 == '\0') break;
          piVar8 = lpProcName;
          piVar17 = lpProcName;
          do {
            piVar18 = piVar17;
            if (piVar8 == (int *)0x0) break;
            piVar8 = (int *)((int)piVar8 + -1);
            piVar18 = (int *)((int)piVar17 + 1);
            iVar4 = *piVar17;
            piVar17 = piVar18;
          } while ((char)((char)iVar7 + -1) != (char)iVar4);
          pFVar6 = GetProcAddress(hModule,(LPCSTR)lpProcName);
          if (pFVar6 == (FARPROC)0x0) {
            return (ulonglong)param_2 << 0x20;
          }
          *puVar14 = pFVar6;
          puVar14 = puVar14 + 1;
        }
      } while( true );
    }
    bVar21 = false;
    pcVar15 = pcVar16;
    if (*pcVar16 == '\x01') {
      uVar3 = *(undefined4 *)pcVar16;
      bVar21 = &DAT_10001000 +
               (CONCAT31(CONCAT21((ushort)uVar3 >> 8,(char)((uint)uVar3 >> 0x10)),
                         (char)((uint)uVar3 >> 0x18)) - (int)pcVar16) == (undefined1 *)0x0;
      pcVar15 = pcVar16 + 4;
      *(undefined1 **)pcVar16 =
           &DAT_10001000 +
           (CONCAT31(CONCAT21((ushort)uVar3 >> 8,(char)((uint)uVar3 >> 0x10)),
                     (char)((uint)uVar3 >> 0x18)) - (int)pcVar16);
    }
  }
  goto LAB_100063c4;
}


