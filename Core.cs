using System.Formats.Tar;
using System.IO.Compression;
using ShellProgressBar;
using YamlDotNet.RepresentationModel;

namespace ununitypackage;

public class Core
{
    static IEnumerable<TarEntry> ToEnumerable(TarReader reader)
    {
        while (true)
        {
            var next = reader.GetNextEntry(true);
            if (next == null) break;
            yield return next;
        }
    }

    static string GetParentPath(string path)
    {
        var parent = Path.GetDirectoryName(path);
        if (string.IsNullOrEmpty(parent)) return path;
        return parent;
    }

    public static bool Extract(FileInfo packpath, string output = ".")
    {
        if (!packpath.Exists)
        {
            Console.WriteLine("The UnityPackage file does not exist.");
            return false;
        }

        // check output path existed or not
        if (!Directory.Exists(output))
        {
            Console.WriteLine("The output directory does not exist.");
            return false;
        }

        // convert output to abs path
        output = Path.GetFullPath(output);
        Console.WriteLine($"Will save files to: {output}");
        // get a one-time temp folder to cache files auto cleanup
        var tempFolder = Path.Combine(Path.GetTempPath(), Path.GetRandomFileName());
        Directory.CreateDirectory(tempFolder);
        try
        {
            using var bar = new ProgressBar(5, "Preparing...", new ProgressBarOptions
            {
                CollapseWhenFinished = true,
                DisplayTimeInRealTime = true
            });
            bar.Tick("Listing package...");
            var count = 0;
            {
                using var tmpfile = packpath.OpenRead();
                using var tmpgzip = new GZipStream(tmpfile, CompressionMode.Decompress);
                using var tmptar = new TarReader(tmpgzip, true);
                count = ToEnumerable(tmptar).Count();
            }
            bar.WriteLine($"Found {count} entries in the archive.");
            using var file = packpath.OpenRead();
            using var gzip = new GZipStream(file, CompressionMode.Decompress);
            using var tar = new TarReader(gzip);

            var package = new UnityPackageStructure();
            var folderToCreate = new List<string>();
            bar.Tick("Collecting assets...");
            // list all folder from root
            {
                using var readbar = bar.Spawn(count, "Collecting...", new ProgressBarOptions
                {
                    CollapseWhenFinished = true,
                    DisplayTimeInRealTime = true
                });
                foreach (var entry in ToEnumerable(tar))
                {
                    if (entry.EntryType != TarEntryType.Directory)
                    {
                        if (entry.Name.EndsWith("pathname"))
                        {
                            var uuid = GetParentPath(entry.Name);
                            readbar.Tick($"[{package.Assets.Count}] Reading (+--) {uuid}...");
                            // try read entry as text
                            var tempFile = Path.Combine(tempFolder, uuid + ".pathname");
                            entry.ExtractToFile(tempFile, true);
                            // read first line as path
                            var path = File.ReadLines(tempFile).First();
                            var parent = GetParentPath(path);
                            if (!folderToCreate.Contains(parent))
                            {
                                folderToCreate.Add(parent);
                            }

                            package.CreateOrUpdateAsset(uuid, path);
                            // remove temp file
                            File.Delete(tempFile);
                        }
                        else if (entry.Name.EndsWith("asset.meta"))
                        {
                            var uuid = GetParentPath(entry.Name);
                            readbar.Tick($"[{package.Assets.Count}] Reading (-+-) {uuid}...");
                            var asset = package.GetAsset(uuid);
                            asset.MetaEntry = entry;
                            asset.MetaSet = true;
                        }
                        else if (entry.Name.EndsWith("asset"))
                        {
                            var uuid = GetParentPath(entry.Name);
                            readbar.Tick($"[{package.Assets.Count}] Reading (--+) {uuid}...");
                            var asset = package.GetAsset(uuid);
                            asset.AssetEntry = entry;
                            asset.AssetSet = true;
                        }
                        else
                        {
                            if (!entry.Name.EndsWith("preview.png") && !entry.Name.EndsWith(".icon.png"))
                                readbar.WriteErrorLine("!!! Entry not recognized: " + entry.Name);
                            readbar.Tick($"[{package.Assets.Count}] Skipping unrecognized entry...");
                        }
                    }
                }
            }


            bar.WriteLine($"{package.Assets.Count} assets found in the package.");
            bar.Tick("Creating structure...");

            {
                using var folderBar = bar.Spawn(folderToCreate.Count, "Creating folders...", new ProgressBarOptions
                {
                    CollapseWhenFinished = true,
                    DisplayTimeInRealTime = true
                });
                // create folders
                foreach (var path in folderToCreate.Select(folder => Path.Combine(output, folder))
                             .Where(path => !Directory.Exists(path)))
                {
                    folderBar.Tick($"Creating {path}...");
                    Directory.CreateDirectory(path);
                }
            }

            bar.Tick("Extracting files...");
            package.DoExtract(output, bar);
            bar.Tick("Done!");
        }
        catch (Exception e)
        {
            Console.WriteLine($"Failed to extract: {e.Message}");
            return false;
        }
        finally
        {
            // delete temp folder
            Directory.Delete(tempFolder, true);
        }

        return true;
    }

    public static bool List(string packpath)
    {
        return false;
    }

    public static bool Build(DirectoryInfo folderpath, FileInfo outputPack, FileInfo cover = null)
{
    // Check if the directory exists
    if (!folderpath.Exists)
    {
        Console.WriteLine("The specified directory does not exist.");
        return false;
    }
    
    using var bar = new ProgressBar(5, "Preparing...", new ProgressBarOptions
    {
        CollapseWhenFinished = true,
        DisplayTimeInRealTime = true
    });

    // Create an instance of UnityPackageStructure
    var package = new UnityPackageStructure();

    bar.Tick("Scanning files...");
    // Get all files in the directory and its subdirectories
    var files = Directory.GetFiles(folderpath.FullName, "*.*", SearchOption.AllDirectories);

    bar.Tick("Collecting assets...");
    
    {
        using var readbar = bar.Spawn(files.Length, "Collecting...", new ProgressBarOptions
        {
            CollapseWhenFinished = true,
            DisplayTimeInRealTime = true
        });
        
        // For each file, create or update an AssetFile in the UnityPackageStructure instance
        foreach (var file in files)
        {
            // If it's a meta file, parse it to get the GUID
            if (file.EndsWith(".meta"))
            {
                using var reader = new StreamReader(file);
                var yaml = new YamlStream();
                yaml.Load(reader);
                var mapping = (YamlMappingNode)yaml.Documents[0].RootNode;
                var guid = mapping.Children[new YamlScalarNode("guid")].ToString();
                var nometa = file.Replace(".meta", "");
                var isFolder = mapping.Children.ContainsKey(new YamlScalarNode("folderAsset"));
                if (!isFolder)
                {
                    isFolder = Directory.Exists(nometa);
                }
                
                // Get the relative path of the file
                var relativePath = Path.GetRelativePath(folderpath.FullName, nometa);
                
                relativePath = Path.Combine("Assets", relativePath);

                // Create or update the asset
                package.CreateOrUpdateAsset(guid, relativePath);
                // readbar.WriteLine($"Got {file} ({relativePath})");
                var asset = package.GetAsset(guid);
                asset.IsFolder = isFolder;
                asset.MetaSet = true;
                asset.PhysicalPath = nometa;
                readbar.Tick($"Got {file}...");
            }
            readbar.Tick();
        }
    }
    // If cover is not null and exists, convert it to string, else pass an empty string
    var coverPath = cover != null && cover.Exists ? cover.FullName : "";
    
    bar.Tick("Building package...");

    package.DoBuildPack(outputPack.FullName, coverPath, bar);
    
    bar.Tick("Done");

    return true;
}
}