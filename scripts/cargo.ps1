param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CargoArgs
)

$vs = 'C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat'
$qt = 'C:\Qt\6.12.0\msvc2022_64\bin'

$command = 'call "{0}" >nul && set "PATH={1};%PATH%" && cargo {2}' -f $vs, $qt, ($CargoArgs -join ' ')

cmd /c $command
exit $LASTEXITCODE
