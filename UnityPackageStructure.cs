using System.Formats.Tar;
using System.IO.Compression;
using ShellProgressBar;
using SharpCompress.Common;
using SharpCompress.Writers;
using SharpCompress.Writers.Tar;
using System.Text;

namespace ununitypackage;

public class UnityPackageStructure
{
    public class AssetFile
    {
        public string UUID;
        public string RealPath = "";
        public string RealMetaPath => $"{RealPath}.meta";
        public TarEntry MetaEntry;
        public TarEntry AssetEntry;
        public bool PathSet = false;
        public bool MetaSet = false;
        public bool AssetSet = false;
        public string PhysicalPath = "";
        public bool IsFolder = false;
        public bool Ready => PathSet && MetaSet && AssetSet;
        public bool MetaReady => PathSet && MetaSet;
        public bool AssetReady => PathSet && AssetSet;

        public AssetFile(string uuid)
        {
            UUID = uuid;
        }
    }

    public Dictionary<string, AssetFile> Assets;

    public UnityPackageStructure()
    {
        Assets = new Dictionary<string, AssetFile>();
    }

    public void CreateOrUpdateAsset(string uuid, string path = "")
    {
        if (!Assets.TryAdd(uuid, new AssetFile(uuid)
            {
                RealPath = path,
                PathSet = true
            }))
        {
            Assets[uuid].RealPath = path;
            Assets[uuid].PathSet = true;
        }
    }

    public AssetFile GetAsset(string uuid)
    {
        if (Assets.TryGetValue(uuid, out var asset))
        {
            return asset;
        }
        else
        {
            var assetItem = new AssetFile(uuid);
            Assets.Add(uuid, assetItem);
            return assetItem;
        }
    }

    public void DoBuildPack(string outputPackage, string cover = "", ProgressBar progressBar = null)
    {
        using var bar = progressBar?.Spawn(Assets.Count + 1, "Building package...", new ProgressBarOptions
        {
            CollapseWhenFinished = true,
            DisplayTimeInRealTime = true
        });
        {
            using var fileStream = File.Create(outputPackage);
            using var tarWriter = WriterFactory.Open(fileStream, ArchiveType.Tar,
                new TarWriterOptions(CompressionType.GZip, true));

            bar?.WriteLine("Streams ready.");
            // If cover is a valid PNG image, add it to the root of the package named ".cover.png"
            if (File.Exists(cover) && Path.GetExtension(cover).Equals(".png", StringComparison.OrdinalIgnoreCase))
            {
                tarWriter.Write(".cover.png", cover);
            }

            bar?.WriteLine("Cover added.");
            // For each AssetFile in the Assets dictionary
            foreach (var asset in Assets.Values)
            {
                // bar?.WriteLine(
                //     $"Adding: {asset.RealPath} (HaveMeta: ${asset.MetaReady}; IsFolder: ${asset.IsFolder})...");
                // In each directory, add three files: "asset", "pathname", and "asset.meta"
                if (!asset.IsFolder) tarWriter.Write(Path.Combine(asset.UUID, "asset"), asset.PhysicalPath);
                tarWriter.Write(Path.Combine(asset.UUID, "pathname"),
                    new MemoryStream(Encoding.UTF8.GetBytes(asset.RealPath + "\n00")));
                tarWriter.Write(Path.Combine(asset.UUID, "asset.meta"), asset.PhysicalPath + ".meta");
                bar?.Tick($"Added {asset.RealPath}...");
            }
        }

        bar?.WriteLine("Assets added.");

        File.Move(outputPackage, Path.ChangeExtension(outputPackage, ".unitypackage"));
    }

    public void DoExtract(string outputBasePath, ProgressBar progressBar)
    {
        var subbar = progressBar.Spawn(Assets.Count, "Extracting assets...", new ProgressBarOptions
        {
            CollapseWhenFinished = true,
            DisplayTimeInRealTime = true
        });
        foreach (var asset in Assets.Values)
        {
            if (asset.Ready)
            {
                var metaPath = Path.Combine(outputBasePath, asset.RealMetaPath);
                var assetPath = Path.Combine(outputBasePath, asset.RealPath);
                asset.MetaEntry.ExtractToFile(metaPath, true);
                asset.AssetEntry.ExtractToFile(assetPath, true);
                subbar?.Tick($"Extracted {asset.RealPath}...");
            }
            else if (asset.MetaReady)
            {
                var metaPath = Path.Combine(outputBasePath, asset.RealMetaPath);
                asset.MetaEntry.ExtractToFile(metaPath, true);
                subbar?.Tick($"Extracted (Folder) {asset.RealPath}...");
            }
            else if (asset.AssetReady)
            {
                var assetPath = Path.Combine(outputBasePath, asset.RealPath);
                asset.AssetEntry.ExtractToFile(assetPath, true);
                subbar?.Tick($"Extracted (PureFile) {asset.RealPath}...");
            }
            else
            {
                var missing = "";
                if (!asset.PathSet) missing += "path, ";
                if (!asset.MetaSet) missing += "meta, ";
                if (!asset.AssetSet) missing += "asset, ";
                subbar?.WriteErrorLine(
                    $"!!! Asset {asset.UUID} ({asset.RealPath}) is not ready to extract. Maybe some files are missing. (missing flag: {missing})");
                subbar?.Tick($"Skipping {asset.UUID}...");
            }
        }
    }
}