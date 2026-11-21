
Run the script from the project root:
```
powershell -ExecutionPolicy Bypass -File scripts\decompile-wmode.ps1
```

Expected output:
```
PS <repo>> powershell -ExecutionPolicy Bypass -File scripts\decompile-wmode.ps1
==> decompiling wmode.bwl (minutes; first run also compiles the postScript)
    INFO  DecompileAll.java> functions=1 failed=0 (GhidraScript)
done: decomp\wmode.dll.c, decomp\wmode.dll.strings
```
