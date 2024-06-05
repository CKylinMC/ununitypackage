using System;
using System.CommandLine;
using System.CommandLine.Invocation;
using ununitypackage;

var app = new RootCommand()
{
    Name = "UnUnityPackage",
    Description = "A tool to extract UnityPackage files without Unity Editor.",
};

var extractCommand = new Command("extract", "Extracts UnityPackage files");
var packfile = new Argument<FileInfo?>("package", "The UnityPackage file to extract");
var outputpath = new Option<string>(new[] { "--output", "-o" }, () => ".", "The output directory");

extractCommand.AddArgument(packfile);
extractCommand.AddOption(outputpath);

extractCommand.SetHandler((package, options) =>
{
    if (package != null)
    {
        Console.WriteLine(Core.Extract(package, options)
            ? "Extracted successfully."
            : "Failed to extract."); // return success or not
    }
    else
    {
        Console.WriteLine("Please provide a UnityPackage file to extract.");
    }
}, packfile, outputpath);

// var listCommand = new Command("list", "Lists the contents of a UnityPackage file")
// {
//     new Argument<FileInfo?>("package", "The UnityPackage file to list"),
// };

var buildCommand = new Command("build", "Builds a UnityPackage file");

var sourcePath = new Argument<DirectoryInfo?>("folder", "Folder to build package");
var output = new Argument<FileInfo?>("output", "Output UnityPackage file");
var cover = new Option<FileInfo?>(new[] { "--cover", "-c" }, "Cover image for the package");

buildCommand.AddArgument(sourcePath);
buildCommand.AddArgument(output);
buildCommand.AddOption(cover);
buildCommand.SetHandler((sourcePath, output, cover) =>
{
    if(sourcePath == null)
    {
        Console.WriteLine("Please provide a folder to build package.");
        return;
    }
    if(output == null)
    {
        Console.WriteLine("Please provide an output file.");
        return;
    }
    Console.WriteLine(Core.Build(sourcePath, output, cover)
        ? "Built successfully."
        : "Failed to build."); // return success or not
}, sourcePath, output, cover);

app.AddCommand(extractCommand);
// app.AddCommand(listCommand);
app.AddCommand(buildCommand);

app.InvokeAsync(args).Wait();