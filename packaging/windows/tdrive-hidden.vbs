' Starts the TDrive web server without a console window. The logon scheduled
' task points at this script; the browser is left alone so boot is quiet.
Set shell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")
script = fso.GetParentFolderName(WScript.ScriptFullName) & "\tdrive-web.cmd"
shell.Run """" & script & """", 0, False
