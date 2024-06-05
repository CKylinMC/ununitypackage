# Uncompress "unitypackage"

A simple tool to extract files from .unitypackage.

## Usage

*I think the built-in help are clear enough so I just copied it here. If you have questions just let me know.*

### Extract

```plain
Description:
  Extracts UnityPackage files

Usage:
  UnUnityPackage extract <package> [options]

Arguments:
  <package>  The UnityPackage file to extract

Options:
  -o, --output <output>  The output directory [default: .]
  -?, -h, --help         Show help and usage information
```

### Build

*This is not stable yet, and have many known issues like resources showing twice in Unity Editor.*

```batch
Description:
  Builds a UnityPackage file

Usage:
  UnUnityPackage build <folder> <output> [options]

Arguments:
  <folder>  Folder to build package
  <output>  Output UnityPackage file

Options:
  -c, --cover <cover>  Cover image for the package
  -?, -h, --help       Show help and usage information
```

## Other

I wrote this tool just I need it during worksing. I'm not so farmilier with unitypackage structure in fact. If you have any suggestions or issues, please let me know. I will try to fix it as soon as possible.

Unity is a trademark of Unity Technologies. This project is not affiliated with Unity Technologies in any way.

