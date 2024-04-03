{
  lib,
  # build dependencies
  runCommandLocal,
  sassc,
  # options
  sasscArguments ? ["-t expanded"],
  styleSheetPath ? ./assets/default-styles.scss,
}:
runCommandLocal "sys-docs-style.css" {nativeBuildInputs = [sassc];} ''
  sassc ${lib.concatStringsSep " " sasscArguments} ${styleSheetPath} > $out
''
