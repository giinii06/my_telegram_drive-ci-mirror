' Starts the TDrive backup agent without a console window. The logon Run
' entry points at this script, so signing in starts the archive quietly.
Set shell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
script = fso.GetParentFolderName(WScript.ScriptFullName) & "\tdrive-backup.cmd"
shell.Run """" & script & """", 0, False
