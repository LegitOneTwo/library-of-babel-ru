#!/usr/bin/env bash
# Экспорт MSVC-окружения в текущий bash-сеанс.
# Использование: source /путь/к/build-env.sh   (из корня проекта)
#
# Три ловушки Windows + Git Bash:
#  1) .bat нужно запускать с абсолютным путём и MSYS_NO_PATHCONV=1, иначе cmd
#     открывает интерактивную оболочку вместо выполнения скрипта.
#  2) .bat должен быть сохранён с CRLF — с LF cmd не исполняет call.
#  3) MSVC отдаёт пути с обратными слэшами: их нужно привести к forward-slash,
#     иначе cargo подхватывает /usr/bin/link из coreutils вместо link.exe.
#
# Скрипт идемпотентен: повторный source не раздувает PATH и INCLUDE/LIB.
# Windows ограничивает длину переменной окружения 32 КБ — при накоплении
# «cmd.exe /c ... set» начинает возвращать пустое окружение.

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
DUMP_BAT="$SCRIPT_DIR/tools/dump-msvc-env.bat"

VCVARS="/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Auxiliary/Build/vcvars64.bat"
if [ ! -f "$VCVARS" ]; then
  echo "vcvars64.bat не найден: $VCVARS" >&2
  return 1 2>/dev/null || exit 1
fi
if [ ! -f "$DUMP_BAT" ]; then
  echo "dump-msvc-env.bat не найден: $DUMP_BAT" >&2
  return 1 2>/dev/null || exit 1
fi

# Перед вызовом cmd сбрасываем то, что могло остаться от прошлого source.
# Иначе cmd наследует уже экспортированные INCLUDE/LIB и дописывает свои
# значения поверх — переменная растёт до лимита Windows (32 КБ), и cmd
# перестаёт отдавать окружение вовсе.
unset INCLUDE LIB LIBPATH VCToolsInstallDir VCINSTALLDIR WindowsSdkDir WindowsSDKVersion

MSYS_ENV=$(MSYS_NO_PATHCONV=1 cmd.exe /c "$(cygpath -w "$DUMP_BAT")" 2>/dev/null | tr -d '\r')

if ! printf '%s' "$MSYS_ENV" | grep -q "^VCToolsInstallDir="; then
  echo "Не удалось получить MSVC-окружение" >&2
  return 1 2>/dev/null || exit 1
fi

export INCLUDE=$(printf '%s\n' "$MSYS_ENV" | sed -n 's/^INCLUDE=//p')
export LIB=$(printf '%s\n' "$MSYS_ENV" | sed -n 's/^LIB=//p')
export LIBPATH=$(printf '%s\n' "$MSYS_ENV" | sed -n 's/^LIBPATH=//p')
export VCToolsInstallDir=$(printf '%s\n' "$MSYS_ENV" | sed -n 's/^VCToolsInstallDir=//p')
export VCINSTALLDIR=$(printf '%s\n' "$MSYS_ENV" | sed -n 's/^VCINSTALLDIR=//p')
export WindowsSdkDir=$(printf '%s\n' "$MSYS_ENV" | sed -n 's/^WindowsSdkDir=//p')
export WindowsSDKVersion=$(printf '%s\n' "$MSYS_ENV" | sed -n 's/^WindowsSDKVersion=//p')

# forward-slash версии для PATH
vc_tools_bin=$(cygpath -u "$VCToolsInstallDir" 2>/dev/null || echo "")
sdk_bin=$(cygpath -u "$WindowsSdkDir" 2>/dev/null || echo "")

# Идемпотентность: чистим PATH от всего, что мы уже добавляли.
# Удаляем и дубликаты вообще (не только наши), иначе PATH растёт при каждом source.
_path_clean=$(printf '%s' "$PATH" | tr ':' '\n' | awk 'NF' | awk '!seen[$0]++' | tr '\n' ':')
PATH=$(printf '%s' "$_path_clean" | tr ':' '\n' | grep -vF -e "$vc_tools_bin/bin/Hostx64/x64" -e "$sdk_bin/bin/$WindowsSDKVersion/x64" -e "$USERPROFILE/.cargo/bin" | awk 'NF' | tr '\n' ':')

PATH="$vc_tools_bin/bin/Hostx64/x64:$sdk_bin/bin/$WindowsSDKVersion/x64:$USERPROFILE/.cargo/bin:$PATH"
export PATH

# Каталог сборки. По умолчанию стандартный target внутри проекта, но его
# можно переопределить переменной окружения: на этой машине Defender
# лочит .cargo-artifact-lock в Documents, поэтому каталог выносят наружу.
# Cargo ждёт нативный путь, а SCRIPT_DIR в Git Bash это MSYS-путь,
# поэтому приводим через cygpath, иначе получится C:\c\Users\...
_default_target="$(cygpath -w "$SCRIPT_DIR/src-tauri/target" 2>/dev/null || echo "$SCRIPT_DIR/src-tauri/target")"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$_default_target}"
mkdir -p "$CARGO_TARGET_DIR"

# Проверка, что link.exe теперь из MSVC, а не из coreutils.
_link=$(command -v link.exe 2>/dev/null)
case "$_link" in
  *msvc*|*MSVC*|*Hostx64*) : ;;
  *) echo "ВНИМАНИЕ: link.exe берётся не из MSVC: $_link" >&2 ;;
esac
