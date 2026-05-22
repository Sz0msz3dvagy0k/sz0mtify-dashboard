import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const preferencesPlugin = join(
	root,
	'node_modules/@capacitor/preferences/ios/Sources/PreferencesPlugin/PreferencesPlugin.swift'
);
const filesystemSources = join(
	root,
	'node_modules/@capacitor/filesystem/ios/Sources/FilesystemPlugin'
);
const nativeAudioIndexTypes = join(root, 'node_modules/@capgo/native-audio/dist/esm/index.d.ts');
const nativeAudioWebTypes = join(root, 'node_modules/@capgo/native-audio/dist/esm/web.d.ts');
const nativeAudioSwiftPlugin = join(
	root,
	'node_modules/@capgo/native-audio/ios/Sources/NativeAudioPlugin/Plugin.swift'
);

const patchedPreferences = `import Foundation
import Capacitor

@objc(PreferencesPlugin)
public class PreferencesPlugin: CAPPlugin, CAPBridgedPlugin {
    public let identifier = "PreferencesPlugin"
    public let jsName = "Preferences"
    public let pluginMethods: [CAPPluginMethod] = [
        CAPPluginMethod(name: "configure", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "get", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "set", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "remove", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "keys", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "clear", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "migrate", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "removeOld", returnType: CAPPluginReturnPromise)
    ]
    private var preferences = Preferences(with: PreferencesConfiguration())

    @objc func configure(_ call: CAPPluginCall) {
        let group = stringValue(call, "group")
        let configuration: PreferencesConfiguration

        if let group = group {
            if group == "NativeStorage" {
                configuration = PreferencesConfiguration(for: .cordovaNativeStorage)
            } else {
                configuration = PreferencesConfiguration(for: .named(group))
            }
        } else {
            configuration = PreferencesConfiguration()
        }

        preferences = Preferences(with: configuration)
        call.resolve()
    }

    @objc func get(_ call: CAPPluginCall) {
        guard let key = stringValue(call, "key") else {
            call.resolve([
                "value": NSNull()
            ])
            return
        }

        let value = preferences.get(by: key)

        call.resolve([
            "value": value as Any
        ])
    }

    @objc func set(_ call: CAPPluginCall) {
        guard let key = stringValue(call, "key") else {
            call.resolve()
            return
        }
        let value = stringValue(call, "value") ?? ""

        preferences.set(value, for: key)
        call.resolve()
    }

    @objc func remove(_ call: CAPPluginCall) {
        guard let key = stringValue(call, "key") else {
            call.resolve()
            return
        }

        preferences.remove(by: key)
        call.resolve()
    }

    @objc func keys(_ call: CAPPluginCall) {
        let keys = preferences.keys()

        call.resolve([
            "keys": keys
        ])
    }

    @objc func clear(_ call: CAPPluginCall) {
        preferences.removeAll()
        call.resolve()
    }

    @objc func migrate(_ call: CAPPluginCall) {
        var migrated: [String] = []
        var existing: [String] = []
        let oldPrefix = "_cap_"
        let oldKeys = UserDefaults.standard.dictionaryRepresentation().keys.filter { $0.hasPrefix(oldPrefix) }

        for oldKey in oldKeys {
            let key = String(oldKey.dropFirst(oldPrefix.count))
            let value = UserDefaults.standard.string(forKey: oldKey) ?? ""
            let currentValue = preferences.get(by: key)

            if currentValue == nil {
                preferences.set(value, for: key)
                migrated.append(key)
            } else {
                existing.append(key)
            }
        }

        call.resolve([
            "migrated": migrated,
            "existing": existing
        ])
    }

    @objc func removeOld(_ call: CAPPluginCall) {
        let oldPrefix = "_cap_"
        let oldKeys = UserDefaults.standard.dictionaryRepresentation().keys.filter { $0.hasPrefix(oldPrefix) }
        for oldKey in oldKeys {
            UserDefaults.standard.removeObject(forKey: oldKey)
        }
        call.resolve()
    }

    private func stringValue(_ call: CAPPluginCall, _ key: String) -> String? {
        if let value = call.options[key] as? String, !value.isEmpty {
            return value
        }
        return nil
    }

}
`;

function patchFile(path, patched, marker, label) {
	let source;
	try {
		source = readFileSync(path, 'utf8');
	} catch {
		return;
	}

	if (source === patched) {
		return;
	}

	if (!source.includes(marker)) {
		throw new Error(`Unexpected ${label} layout at ${path}`);
	}

	writeFileSync(path, patched);
	console.log(`Patched ${label} for SwiftPM compatibility`);
}

patchFile(preferencesPlugin, patchedPreferences, 'public class PreferencesPlugin', '@capacitor/preferences iOS source');

function patchTextFile(path, replacements, label) {
	let source;
	try {
		source = readFileSync(path, 'utf8');
	} catch {
		return;
	}

	let patched = source;
	for (const [from, to] of replacements) {
		patched = patched.replace(from, to);
	}
	if (patched !== source) {
		writeFileSync(path, patched);
		console.log(`Patched ${label}`);
	}
}

patchTextFile(
	nativeAudioIndexTypes,
	[
		["import { NativeAudio } from './definitions';", "import type { NativeAudio as NativeAudioPlugin } from './definitions';"],
		["import type { NativeAudio } from './definitions';", "import type { NativeAudio as NativeAudioPlugin } from './definitions';"],
		['declare const NativeAudio: NativeAudio;', 'declare const NativeAudio: NativeAudioPlugin;']
	],
	'@capgo/native-audio TypeScript declarations'
);
patchTextFile(
	nativeAudioWebTypes,
	[
		["import { NativeAudio } from './definitions';", "import type { NativeAudio as NativeAudioPlugin } from './definitions';"],
		["import type { NativeAudio } from './definitions';", "import type { NativeAudio as NativeAudioPlugin } from './definitions';"],
		['export declare class NativeAudioWeb extends WebPlugin implements NativeAudio {', 'export declare class NativeAudioWeb extends WebPlugin implements NativeAudioPlugin {'],
		['declare const NativeAudio: NativeAudioWeb;', 'declare const NativeAudio: NativeAudioWeb;']
	],
	'@capgo/native-audio web TypeScript declarations'
);

function patchNativeAudioSwiftPlugin() {
	let source;
	try {
		source = readFileSync(nativeAudioSwiftPlugin, 'utf8');
	} catch {
		return;
	}

	let patched = source;
const rejectShim = `private extension CAPPluginCall {
    func rejectCompat(_ message: String, _ code: String? = nil, _ error: Error? = nil, _ data: PluginCallResultData? = nil) {
        unavailable(message)
    }
}

`;
	if (!patched.includes('func rejectCompat(')) {
		patched = patched.replace('enum MyError: Error {', `${rejectShim}enum MyError: Error {`);
	}
	patched = patched.replace(
		'errorHandler(CAPPluginCallError(message: message, code: code, error: error, data: data))',
		'unavailable(message)'
	);
	patched = patched.replaceAll('.rejectCall(', '.rejectCompat(');
	patched = patched.replaceAll('.reject(', '.rejectCompat(');
	const replacements = [
		['let debug = call.getBool("enabled") ?? false', 'let debug = call.getBool("enabled", false)'],
		['let focus = call.getBool(Constant.FocusAudio) ?? false', 'let focus = call.getBool(Constant.FocusAudio, false)'],
		['let background = call.getBool(Constant.Background) ?? false', 'let background = call.getBool(Constant.Background, false)'],
		['let ignoreSilent = call.getBool(Constant.IgnoreSilent) ?? true', 'let ignoreSilent = call.getBool(Constant.IgnoreSilent, true)'],
		[
			'if let showNotification = call.getBool(Constant.ShowNotification) {\n            self.showNotification = showNotification\n        }',
			'if call.options[Constant.ShowNotification] != nil {\n            self.showNotification = call.getBool(Constant.ShowNotification, false)\n        }'
		],
		[
			'guard let assetId = call.getString(Constant.AssetIdKey) else {\n            call.rejectCompat("Missing assetId")\n            return\n        }',
			'let assetId = call.getString(Constant.AssetIdKey, "")\n        if assetId.isEmpty {\n            call.rejectCompat("Missing assetId")\n            return\n        }'
		],
		['call.rejectCall("Missing assetId")', 'call.rejectCompat("Missing assetId")'],
		['call.getString(Constant.AssetPathKey) ?? ""', 'call.getString(Constant.AssetPathKey, "")'],
		['call.getString(Constant.AssetIdKey) ?? ""', 'call.getString(Constant.AssetIdKey, "")'],
		['call.getString(Constant.AssetIdKey) ?? ""]', 'call.getString(Constant.AssetIdKey, "")]'],
		['call.getBool("autoPlay") ?? true', 'call.getBool("autoPlay", true)'],
		['call.getBool("deleteAfterPlay") ?? false', 'call.getBool("deleteAfterPlay", false)'],
		['call.getBool("isUrl") ?? false', 'call.getBool("isUrl", false)'],
		['call.getBool(Constant.FadeIn) ?? false', 'call.getBool(Constant.FadeIn, false)'],
		['call.getBool(Constant.FadeOut) ?? false', 'call.getBool(Constant.FadeOut, false)'],
		['call.getDouble(Constant.Time) ?? 0', 'call.getDouble(Constant.Time, 0)'],
		['call.getDouble(Constant.Delay) ?? 0', 'call.getDouble(Constant.Delay, 0)'],
		['call.getDouble(Constant.FadeInDuration) ?? Double(Constant.DefaultFadeDuration)', 'call.getDouble(Constant.FadeInDuration, Double(Constant.DefaultFadeDuration))'],
		['call.getDouble(Constant.FadeOutDuration) ?? Double(Constant.DefaultFadeDuration)', 'call.getDouble(Constant.FadeOutDuration, Double(Constant.DefaultFadeDuration))'],
		['call.getDouble(Constant.FadeOutStartTime) ?? 0.0', 'call.getDouble(Constant.FadeOutStartTime, 0.0)'],
		['call.getDouble(Constant.FadeDuration) ?? 0.0', 'call.getDouble(Constant.FadeDuration, 0.0)'],
		['call.getFloat("volume") ?? Constant.DefaultVolume', 'call.getFloat("volume", Constant.DefaultVolume)'],
		['call.getFloat(Constant.Volume) ?? Constant.DefaultVolume', 'call.getFloat(Constant.Volume, Constant.DefaultVolume)'],
		['call.getFloat(Constant.Rate) ?? Constant.DefaultRate', 'call.getFloat(Constant.Rate, Constant.DefaultRate)'],
		['call.getInt("channels") ?? Constant.DefaultChannels', 'call.getInt("channels", Constant.DefaultChannels)'],
		[
			'let volume = call.getFloat(Constant.Volume)',
			'let volume = call.options[Constant.Volume] != nil ? call.getFloat(Constant.Volume, Constant.DefaultVolume) : nil'
		],
		[
			'if let metadata = call.getObject(Constant.NotificationMetadata) {',
			'let metadata = call.getObject(Constant.NotificationMetadata, [:])\n        if !metadata.isEmpty {'
		],
		[
			'if let headersObj = call.getObject("headers") {',
			'let headersObj = call.getObject("headers", [:])\n                    if !headersObj.isEmpty {'
		],
		['commandCenter.skipForwardCommand.preferredIntervals = [NSNumber(value: 15)]', 'commandCenter.skipForwardCommand.preferredIntervals = []'],
		['commandCenter.skipForwardCommand.isEnabled = true', 'commandCenter.skipForwardCommand.isEnabled = false'],
		['commandCenter.skipBackwardCommand.preferredIntervals = [NSNumber(value: 15)]', 'commandCenter.skipBackwardCommand.preferredIntervals = []'],
		['commandCenter.skipBackwardCommand.isEnabled = true', 'commandCenter.skipBackwardCommand.isEnabled = false'],
		['audioQueue.sync {\n            guard !audioList.isEmpty else {', 'audioQueue.sync { [self] in\n            guard !audioList.isEmpty else {'],
		[
			'self.updateNowPlayingInfo(audioId: assetId, audioAsset: asset)\n                    self.updatePlaybackState(isPlaying: true)',
			'self.updateNowPlayingInfo(audioId: assetId, audioAsset: asset)\n                    self.updatePlaybackState(isPlaying: true, elapsedTime: asset.getCurrentTime(), duration: asset.getDuration())'
		],
		[
			'self.updateNowPlayingInfo(audioId: audioId, audioAsset: audioAsset)\n                            self.updatePlaybackState(isPlaying: true)',
			'self.updateNowPlayingInfo(audioId: audioId, audioAsset: audioAsset)\n                            self.updatePlaybackState(isPlaying: true, elapsedTime: audioAsset.getCurrentTime(), duration: audioAsset.getDuration())'
		],
		[
			'self.updateNowPlayingInfo(audioId: audioAsset.assetId, audioAsset: audioAsset)\n                self.updatePlaybackState(isPlaying: true)',
			'self.updateNowPlayingInfo(audioId: audioAsset.assetId, audioAsset: audioAsset)\n                self.updatePlaybackState(isPlaying: true, elapsedTime: audioAsset.getCurrentTime(), duration: audioAsset.getDuration())'
		],
		[
			'let time = max(call.getDouble(Constant.Time, 0), 0)\n            audioAsset.setCurrentTime(time: time) {\n                call.resolve()\n            }',
			'let time = max(call.getDouble(Constant.Time, 0), 0)\n            audioAsset.setCurrentTime(time: time) { [weak self] in\n                guard let self else {\n                    call.resolve()\n                    return\n                }\n                if self.showNotification && self.currentlyPlayingAssetId == audioAsset.assetId {\n                    self.updatePlaybackState(isPlaying: audioAsset.isPlaying(), elapsedTime: time, duration: audioAsset.getDuration())\n                }\n                self.notifyPlaybackState(assetId: audioAsset.assetId, reason: "seek", audioAsset: audioAsset)\n                call.resolve()\n            }'
		],
		[
			'notifyListeners("currentTime", data: [\n                "currentTime": currentTime,\n                "assetId": asset.assetId\n            ])',
			'notifyListeners("currentTime", data: [\n                "currentTime": currentTime,\n                "assetId": asset.assetId\n            ])\n\n            if showNotification && currentlyPlayingAssetId == asset.assetId {\n                let duration = asset.getDuration()\n                updatePlaybackState(\n                    isPlaying: asset.isPlaying(),\n                    elapsedTime: currentTime,\n                    duration: duration.isFinite && duration > 0 ? duration : nil\n                )\n            }'
		]
	];

	for (const [from, to] of replacements) {
		patched = patched.replaceAll(from, to);
	}

	const nextPreviousRemoteCommands = `        // Next track command
        commandCenter.nextTrackCommand.isEnabled = true
        commandCenter.nextTrackCommand.addTarget { [weak self] _ in
            guard let self = self, let assetId = self.currentlyPlayingAssetId else {
                return .noSuchContent
            }
            self.notifyPlaybackState(assetId: assetId, reason: "remoteNext")
            return .success
        }

        // Previous track command
        commandCenter.previousTrackCommand.isEnabled = true
        commandCenter.previousTrackCommand.addTarget { [weak self] _ in
            guard let self = self, let assetId = self.currentlyPlayingAssetId else {
                return .noSuchContent
            }
            self.notifyPlaybackState(assetId: assetId, reason: "remotePrevious")
            return .success
        }

`;
	if (!patched.includes('commandCenter.nextTrackCommand.addTarget')) {
		patched = patched.replace('        // Skip forward command', `${nextPreviousRemoteCommands}        // Skip forward command`);
	}

	if (patched !== source) {
		writeFileSync(nativeAudioSwiftPlugin, patched);
		console.log('Patched @capgo/native-audio Swift source for Capacitor SwiftPM compatibility');
	}
}

patchNativeAudioSwiftPlugin();

const patchedFilesystemAccelerators = `import Capacitor
import Foundation
import IONFilesystemLib
import ObjectiveC

extension CAPPluginCall {
    func getEncoding(_ key: String) -> IONFILEEncoding {
        if let encodingParameter = stringValue(key) {
            .string(encoding: .create(from: encodingParameter))
        } else {
            .byteBuffer
        }
    }

    func getSearchPath(_ key: String) -> IONFILESearchPath {
        getSearchPath(key, withDefaultSearchPath: .raw, andDefaultDirectoryType: .document)
    }

    func getSearchPath(_ key: String, withDefault defaultValue: IONFILESearchPath) -> IONFILESearchPath {
        getSearchPath(key, withDefaultSearchPath: defaultValue)
    }

    func getEncodingMapper() -> IONFILEEncodingValueMapper? {
        guard let data: String = stringValue(Constants.MethodParameter.data) else {
            return nil
        }
        return switch getEncoding(Constants.MethodParameter.encoding) {
        case .byteBuffer:
            if let base64Data = Self.data(base64EncodedOrDataUrl: data) {
                .byteBuffer(value: base64Data)
            } else {
                nil
            }
        case .string(encoding: let stringEncoding):
            .string(encoding: stringEncoding, value: data)
        @unknown default: nil
        }
    }

    func getIONFileMethod() -> IONFileMethod {
        return IONFileMethod(rawValue: self.methodName) ?? IONFileMethod.getUri
    }

    func handleSuccess(_ data: PluginCallResultData?, _ keepCallAlive: Bool = false) {
        keepAlive = keepCallAlive
        if let data {
            resolve(data)
        } else {
            resolve()
        }
    }

    func handlePermissionSuccess() {
        handleSuccess([Constants.ResultDataKey.publicStorage: Constants.ResultDataValue.granted])
    }

    func handleError(_ error: FilesystemError) {
        let errorPair = error.toCodeMessagePair()
        rejectCall(errorPair.message, errorPair.code)
    }

    func stringValue(_ key: String) -> String? {
        if let value = options[key] as? String, !value.isEmpty {
            return value
        }
        return nil
    }

    func stringValue(_ key: String, _ defaultValue: String) -> String {
        stringValue(key) ?? defaultValue
    }

    func intValue(_ key: String) -> Int? {
        if let value = options[key] as? Int {
            return value
        }
        if let value = options[key] as? NSNumber {
            return value.intValue
        }
        if let value = options[key] as? String {
            return Int(value)
        }
        return nil
    }

    func intValue(_ key: String, _ defaultValue: Int) -> Int {
        intValue(key) ?? defaultValue
    }

    func boolValue(_ key: String, _ defaultValue: Bool) -> Bool {
        if let value = options[key] as? Bool {
            return value
        }
        if let value = options[key] as? NSNumber {
            return value.boolValue
        }
        if let value = options[key] as? String {
            return ["true", "1", "yes"].contains(value.lowercased())
        }
        return defaultValue
    }

    func rejectCall(_ message: String, _ code: String? = nil, _ error: Error? = nil, _ data: PluginCallResultData? = nil) {
        let selector = NSSelectorFromString("reject::::")
        guard let method = class_getInstanceMethod(CAPPluginCall.self, selector) else {
            NSLog("Unable to reject Capacitor filesystem call because the reject selector is unavailable: %@", message)
            return
        }
        typealias RejectFunction = @convention(c) (CAPPluginCall, Selector, NSString, NSString?, NSError?, NSDictionary?) -> Void
        let function = unsafeBitCast(method_getImplementation(method), to: RejectFunction.self)
        function(self, selector, message as NSString, code.map { $0 as NSString }, error as NSError?, data.map { $0 as NSDictionary })
    }

    private static func data(base64EncodedOrDataUrl value: String) -> Data? {
        if value.hasPrefix("data:"), let range = value.range(of: "base64,") {
            return Data(base64Encoded: String(value[range.upperBound...]))
        }
        return Data(base64Encoded: value)
    }

    private func getSearchPath(
        _ key: String, withDefaultSearchPath defaultSearchPath: IONFILESearchPath, andDefaultDirectoryType defaultDirectoryType: IONFILEDirectoryType? = nil
    ) -> IONFILESearchPath {
        guard let directoryParameter = stringValue(key), directoryParameter.isEmpty == false else {
            return defaultSearchPath
        }

        return if let type = IONFILEDirectoryType.create(from: directoryParameter) ?? defaultDirectoryType {
            .directory(type: type)
        } else {
            defaultSearchPath
        }
    }
}
`;

const patchedFilesystemLocationResolver = `import Capacitor
import Foundation
import IONFilesystemLib

struct FilesystemLocationResolver {
    let service: FileService

    func resolveSinglePath(from call: CAPPluginCall) -> Result<URL, FilesystemError> {
        guard let path = call.stringValue(Constants.MethodParameter.path) else {
            return .failure(.invalidInput(method: call.getIONFileMethod()))
        }

        let directory = call.getSearchPath(Constants.MethodParameter.directory)
        return resolveURL(path: path, directory: directory)
    }

    func resolveDualPaths(from call: CAPPluginCall) -> Result<(source: URL, destination: URL), FilesystemError> {
        guard let fromPath = call.stringValue(Constants.MethodParameter.from), let toPath = call.stringValue(Constants.MethodParameter.to) else {
            return .failure(.invalidInput(method: call.getIONFileMethod()))
        }

        let fromDirectory = call.getSearchPath(Constants.MethodParameter.directory)
        let toDirectory = call.getSearchPath(Constants.MethodParameter.toDirectory, withDefault: fromDirectory)

        return resolveURL(path: fromPath, directory: fromDirectory)
            .flatMap { sourceURL in
                resolveURL(path: toPath, directory: toDirectory)
                    .map { (source: sourceURL, destination: $0) }
            }
    }

    private func resolveURL(path: String, directory: IONFILESearchPath) -> Result<URL, FilesystemError> {
        return if let url = try? service.getFileURL(atPath: path, withSearchPath: directory) {
            .success(url)
        } else {
            .failure(.invalidPath(path))
        }
    }
}
`;

const patchedLegacyFilesystem = `import Foundation
import Capacitor

@objc public class LegacyFilesystemImplementation: NSObject {

    public typealias ProgressEmitter = (_ bytes: Int64, _ contentLength: Int64) -> Void

    // swiftlint:disable function_body_length
    @objc public func downloadFile(call: CAPPluginCall, emitter: @escaping ProgressEmitter, config: InstanceConfiguration?) throws {
        let directory = call.stringValue("directory", "DOCUMENTS")
        guard let path = call.stringValue("path") else {
            call.rejectCall("Invalid file path")
            return
        }
        guard var urlString = call.stringValue("url") else { throw URLError(.badURL) }

        func handleDownload(downloadLocation: URL?, response: URLResponse?, error: Error?) {
            if let error = error {
                CAPLog.print("Error on download file", String(describing: downloadLocation), String(describing: response), String(describing: error))
                call.rejectCall(error.localizedDescription, "DOWNLOAD", error)
                return
            }

            if let httpResponse = response as? HTTPURLResponse {
                if !(200...299).contains(httpResponse.statusCode) {
                    CAPLog.print("Error downloading file:", urlString, httpResponse)
                    call.rejectCall("Error downloading file: \\(urlString)", "DOWNLOAD")
                    return
                }
            }

            guard let location = downloadLocation else {
                call.rejectCall("Unable to get file after downloading")
                return
            }

            let fileManager = FileManager.default

            if let foundDir = getDirectory(directory: directory) {
                let dir = fileManager.urls(for: foundDir, in: .userDomainMask).first

                do {
                    let dest = dir!.appendingPathComponent(path)
                    CAPLog.print("Attempting to write to file destination: \\(dest.absoluteString)")

                    if !FileManager.default.fileExists(atPath: dest.deletingLastPathComponent().absoluteString) {
                        try FileManager.default.createDirectory(at: dest.deletingLastPathComponent(), withIntermediateDirectories: true, attributes: nil)
                    }

                    if FileManager.default.fileExists(atPath: dest.relativePath) {
                        do {
                            CAPLog.print("File already exists. Attempting to remove file before writing.")
                            try fileManager.removeItem(at: dest)
                        } catch let error {
                            call.rejectCall("Unable to remove existing file: \\(error.localizedDescription)")
                            return
                        }
                    }

                    try fileManager.moveItem(at: location, to: dest)
                    CAPLog.print("Downloaded file successfully! \\(dest.absoluteString)")
                    call.resolve(["path": dest.absoluteString])
                } catch let error {
                    call.rejectCall("Unable to download file: \\(error.localizedDescription)", "DOWNLOAD", error)
                    return
                }
            } else {
                call.rejectCall("Unable to download file. Couldn't find directory \\(directory)")
            }
        }

        let method = call.stringValue("method", "GET")

        let headers = (call.options["headers"] as? [String: Any]) ?? [:]
        let params = (call.options["params"] as? [String: Any]) ?? [:]
        let connectTimeout = call.options["connectTimeout"] as? Double
        let readTimeout = call.options["readTimeout"] as? Double

        if urlString == urlString.removingPercentEncoding {
            guard let encodedUrlString = urlString.addingPercentEncoding(withAllowedCharacters: CharacterSet.urlQueryAllowed)  else { throw URLError(.badURL) }
            urlString = encodedUrlString
        }

        let progress = (call.options["progress"] as? Bool) ?? false

        let request = try HttpRequestHandler.CapacitorHttpRequestBuilder()
            .setUrl(urlString)
            .setMethod(method)
            .setUrlParams(params)
            .openConnection()
            .build()

        request.setRequestHeaders(headers)

        // Timeouts in iOS are in seconds. So read the value in millis and divide by 1000
        let timeout = (connectTimeout ?? readTimeout ?? 600000.0) / 1000.0
        request.setTimeout(timeout)

        var session: URLSession!
        var task: URLSessionDownloadTask!
        let urlRequest = request.getUrlRequest()

        if progress {
            class ProgressDelegate: NSObject, URLSessionDataDelegate, URLSessionDownloadDelegate {
                private var handler: (URL?, URLResponse?, Error?) -> Void
                private var downloadLocation: URL?
                private var response: URLResponse?
                private var emitter: (Int64, Int64) -> Void
                private var lastEmitTimestamp: TimeInterval = 0.0

                init(downloadHandler: @escaping (URL?, URLResponse?, Error?) -> Void, progressEmitter: @escaping (Int64, Int64) -> Void) {
                    handler = downloadHandler
                    emitter = progressEmitter
                }

                func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didWriteData bytesWritten: Int64, totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64) {
                    let currentTimestamp = Date().timeIntervalSince1970
                    let timeElapsed = currentTimestamp - lastEmitTimestamp

                    if totalBytesExpectedToWrite > 0 {
                        if timeElapsed >= 0.1 {
                            emitter(totalBytesWritten, totalBytesExpectedToWrite)
                            lastEmitTimestamp = currentTimestamp
                        }
                    } else {
                        emitter(totalBytesWritten, 0)
                        lastEmitTimestamp = currentTimestamp
                    }
                }

                func urlSession(_ session: URLSession, downloadTask: URLSessionDownloadTask, didFinishDownloadingTo location: URL) {
                    downloadLocation = location
                    handler(downloadLocation, downloadTask.response, downloadTask.error)
                }

                func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
                    if error != nil {
                        handler(downloadLocation, task.response, error)
                    }
                }
            }

            let progressDelegate = ProgressDelegate(downloadHandler: handleDownload, progressEmitter: emitter)
            session = URLSession(configuration: .default, delegate: progressDelegate, delegateQueue: nil)
            task = session.downloadTask(with: urlRequest)
        } else {
            task = URLSession.shared.downloadTask(with: urlRequest, completionHandler: handleDownload)
        }

        task.resume()
    }
    // swiftlint:enable function_body_length

    /**
     * Get the SearchPathDirectory corresponding to the JS string
     */
    private func getDirectory(directory: String?) -> FileManager.SearchPathDirectory? {
        if let directory = directory {
            switch directory {
            case "CACHE":
                return .cachesDirectory
            case "LIBRARY":
                return .libraryDirectory
            default:
                return .documentDirectory
            }
        }

        return nil
    }
}
`;

const patchedFilesystemPlugin = `import Foundation
import Capacitor
import IONFilesystemLib

typealias FileService = any IONFILEDirectoryManager & IONFILEFileManager

/**
 * Please read the Capacitor iOS Plugin Development Guide
 * here: https://capacitorjs.com/docs/plugins/ios
 */
@objc(FilesystemPlugin)
public class FilesystemPlugin: CAPPlugin, CAPBridgedPlugin {
    public let identifier = "FilesystemPlugin"
    public let jsName = "Filesystem"
    public let pluginMethods: [CAPPluginMethod] = [
        CAPPluginMethod(name: "readFile", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "readFileInChunks", returnType: CAPPluginReturnCallback),
        CAPPluginMethod(name: "writeFile", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "appendFile", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "deleteFile", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "mkdir", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "rmdir", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "readdir", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "getUri", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "stat", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "rename", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "copy", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "checkPermissions", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "requestPermissions", returnType: CAPPluginReturnPromise),
        CAPPluginMethod(name: "downloadFile", returnType: CAPPluginReturnPromise)
    ]

    private let legacyImplementation = LegacyFilesystemImplementation()

    private var fileService: FileService?

    override public func load() {
        self.fileService = IONFILEManager()
    }

    func getService() -> Result<FileService, FilesystemError> {
        if fileService == nil { load() }
        return fileService.map(Result.success) ?? .failure(.bridgeNotInitialised)
    }

    @objc override public func checkPermissions(_ call: CAPPluginCall) {
        call.handlePermissionSuccess()
    }

    @objc override public func requestPermissions(_ call: CAPPluginCall) {
        call.handlePermissionSuccess()
    }
}

// MARK: - Public API Methods
private extension FilesystemPlugin {
    /**
     * Read a file from the filesystem.
     */
    @objc func readFile(_ call: CAPPluginCall) {
        let encoding = call.getEncoding(Constants.MethodParameter.encoding)
        let offset = call.intValue(Constants.MethodParameter.offset, 0)
        let length = call.intValue(Constants.MethodParameter.length, -1)
        performSinglePathOperation(call) {
            .readFile(url: $0, encoding: encoding, offset: offset, length: length)
        }
    }

    @objc func readFileInChunks(_ call: CAPPluginCall) {
        let encoding = call.getEncoding(Constants.MethodParameter.encoding)
        guard let chunkSize = call.intValue(Constants.MethodParameter.chunkSize) else {
            return call.handleError(.invalidInput(method: call.getIONFileMethod()))
        }
        let offset = call.intValue(Constants.MethodParameter.offset, 0)
        let length = call.intValue(Constants.MethodParameter.length, -1)
        performSinglePathOperation(call) {
            .readFileInChunks(url: $0, encoding: encoding, chunkSize: chunkSize, offset: offset, length: length)
        }
    }

    /**
     * Write a file to the filesystem.
     */
    @objc func writeFile(_ call: CAPPluginCall) {
        guard let encodingMapper = call.getEncodingMapper() else {
            return call.handleError(.invalidInput(method: call.getIONFileMethod()))
        }
        let recursive = call.boolValue(Constants.MethodParameter.recursive, false)

        performSinglePathOperation(call) {
            .write(url: $0, encodingMapper: encodingMapper, recursive: recursive)
        }
    }

    /**
     * Append to a file.
     */
    @objc func appendFile(_ call: CAPPluginCall) {
        guard let encodingMapper = call.getEncodingMapper() else {
            return call.handleError(.invalidInput(method: call.getIONFileMethod()))
        }
        let recursive = call.boolValue(Constants.MethodParameter.recursive, false)

        performSinglePathOperation(call) {
            .append(url: $0, encodingMapper: encodingMapper, recursive: recursive)
        }
    }

    /**
     * Delete a file.
     */
    @objc func deleteFile(_ call: CAPPluginCall) {
        performSinglePathOperation(call) {
            .delete(url: $0)
        }
    }

    /**
     * Make a new directory, optionally creating parent folders first.
     */
    @objc func mkdir(_ call: CAPPluginCall) {
        let recursive = call.boolValue(Constants.MethodParameter.recursive, false)

        performSinglePathOperation(call) {
            .mkdir(url: $0, recursive: recursive)
        }
    }

    /**
     * Remove a directory.
     */
    @objc func rmdir(_ call: CAPPluginCall) {
        let recursive = call.boolValue(Constants.MethodParameter.recursive, false)

        performSinglePathOperation(call) {
            .rmdir(url: $0, recursive: recursive)
        }
    }

    /**
     * Read the contents of a directory.
     */
    @objc func readdir(_ call: CAPPluginCall) {
        performSinglePathOperation(call) {
            .readdir(url: $0)
        }
    }

    @objc func stat(_ call: CAPPluginCall) {
        performSinglePathOperation(call) {
            .stat(url: $0)
        }
    }

    @objc func getUri(_ call: CAPPluginCall) {
        performSinglePathOperation(call) {
            .getUri(url: $0)
        }
    }

    /**
     * Rename a file or directory.
     */
    @objc func rename(_ call: CAPPluginCall) {
        performDualPathOperation(call) {
            .rename(source: $0, destination: $1)
        }
    }

    /**
     * Copy a file or directory.
     */
    @objc func copy(_ call: CAPPluginCall) {
        performDualPathOperation(call) {
            .copy(source: $0, destination: $1)
        }
    }

    /**
     * [DEPRECATED] Download a file
     */
    @available(*, deprecated, message: "Use @capacitor/file-transfer plugin instead.")
    @objc func downloadFile(_ call: CAPPluginCall) {
        guard let url = call.stringValue("url") else { return call.rejectCall("Must provide a URL") }
        let progressEmitter: LegacyFilesystemImplementation.ProgressEmitter = { bytes, contentLength in
            self.notifyListeners("progress", data: [
                "url": url,
                "bytes": bytes,
                "contentLength": contentLength
            ])
        }

        do {
            try legacyImplementation.downloadFile(call: call, emitter: progressEmitter, config: bridge?.config)
        } catch let error {
            call.rejectCall(error.localizedDescription)
        }
    }
}

// MARK: - Operation Execution
private extension FilesystemPlugin {
    func performSinglePathOperation(_ call: CAPPluginCall, operationBuilder: (URL) -> FilesystemOperation) {
        executeOperation(call) { service in
            FilesystemLocationResolver(service: service)
                .resolveSinglePath(from: call)
                .map { operationBuilder($0) }
        }
    }

    func performDualPathOperation(_ call: CAPPluginCall, operationBuilder: (URL, URL) -> FilesystemOperation) {
        executeOperation(call) { service in
            FilesystemLocationResolver(service: service)
                .resolveDualPaths(from: call)
                .map { operationBuilder($0.source, $0.destination) }
        }
    }

    func executeOperation(_ call: CAPPluginCall, operationProvider: (FileService) -> Result<FilesystemOperation, FilesystemError>) {
        switch getService() {
        case .success(let service):
            switch operationProvider(service) {
            case .success(let operation):
                let executor = FilesystemOperationExecutor(service: service)
                executor.execute(operation, call)
            case .failure(let error):
                call.handleError(error)
            }
        case .failure(let error):
            call.handleError(error)
        }
    }
}
`;

patchFile(
	join(filesystemSources, 'CAPPluginCall+Accelerators.swift'),
	patchedFilesystemAccelerators,
	'extension CAPPluginCall',
	'@capacitor/filesystem CAPPluginCall accelerators'
);
patchFile(
	join(filesystemSources, 'FilesystemLocationResolver.swift'),
	patchedFilesystemLocationResolver,
	'struct FilesystemLocationResolver',
	'@capacitor/filesystem location resolver'
);
patchFile(
	join(filesystemSources, 'LegacyFilesystemImplementation.swift'),
	patchedLegacyFilesystem,
	'public class LegacyFilesystemImplementation',
	'@capacitor/filesystem legacy implementation'
);
patchFile(
	join(filesystemSources, 'FilesystemPlugin.swift'),
	patchedFilesystemPlugin,
	'public class FilesystemPlugin',
	'@capacitor/filesystem plugin'
);
