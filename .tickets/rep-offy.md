---
id: rep-offy
status: closed
deps: []
links: []
created: 2026-07-23T06:52:35Z
type: bug
priority: 1
assignee: memgrafter
---
# Repair .NET Interactive kernel dependencies

## Notes

**2026-07-23T06:57:57Z**

Conda-forge has only PowerShell 7.5+, while its dotnet-interactive build requires System.Management.Automation 7.4.5. Replaced the broken conda kernel package with dotnet/nodejs plus the official Microsoft.dotnet-interactive NuGet tool. Installer normalizes generated .NET kernelspecs through micromamba. Verified persistent C# state: answer=42 then Console.WriteLine(answer+1) emitted 43.
