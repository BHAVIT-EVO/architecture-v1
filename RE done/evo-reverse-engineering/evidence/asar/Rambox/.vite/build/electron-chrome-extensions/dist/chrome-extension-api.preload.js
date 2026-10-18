"use strict";
(() => {
  var __require = /* @__PURE__ */ ((x) => typeof require !== "undefined" ? require : typeof Proxy !== "undefined" ? new Proxy(x, {
    get: (a, b) => (typeof require !== "undefined" ? require : a)[b]
  }) : x)(function(x) {
    if (typeof require !== "undefined") return require.apply(this, arguments);
    throw Error('Dynamic require of "' + x + '" is not supported');
  });

  // src/renderer/index.ts
  var import_electron2 = __require("electron");

  // src/renderer/event.ts
  var import_electron = __require("electron");
  var formatIpcName = (name) => `crx-${name}`;
  var listenerMap = /* @__PURE__ */ new Map();
  var addExtensionListener = (extensionId, name, callback) => {
    const listenerCount = listenerMap.get(name) || 0;
    if (listenerCount === 0) {
      import_electron.ipcRenderer.send("crx-add-listener", extensionId, name);
    }
    listenerMap.set(name, listenerCount + 1);
    import_electron.ipcRenderer.addListener(formatIpcName(name), function(event, ...args) {
      if (true) {
        console.log(name, "(result)", ...args);
      }
      callback(...args);
    });
  };
  var hasExtensionListeners = (name) => {
    return (listenerMap.get(name) || 0) > 0;
  };
  var removeExtensionListener = (extensionId, name, callback) => {
    if (listenerMap.has(name)) {
      const listenerCount = listenerMap.get(name) || 0;
      if (listenerCount <= 1) {
        listenerMap.delete(name);
        import_electron.ipcRenderer.send("crx-remove-listener", extensionId, name);
      } else {
        listenerMap.set(name, listenerCount - 1);
      }
    }
    import_electron.ipcRenderer.removeListener(formatIpcName(name), callback);
  };

  // src/renderer/index.ts
  var injectExtensionAPIs = () => {
    const invokeExtension = async function(extensionId, fnName, options = {}, ...args) {
      const callback = typeof args[args.length - 1] === "function" ? args.pop() : void 0;
      if (true) {
        console.log(fnName, args);
      }
      if (options.noop) {
        console.warn(`${fnName} is not yet implemented.`);
        if (callback) callback(options.defaultResponse);
        return Promise.resolve(options.defaultResponse);
      }
      if (options.serialize) {
        args = options.serialize(...args);
      }
      let result;
      try {
        result = await import_electron2.ipcRenderer.invoke("crx-msg", extensionId, fnName, ...args);
      } catch (e) {
        console.error(e);
        result = void 0;
      }
      if (true) {
        console.log(fnName, "(result)", result);
      }
      if (callback) {
        callback(result);
      } else {
        return result;
      }
    };
    const connectNative = (extensionId, application, receive, disconnect, callback) => {
      const connectionId = import_electron2.contextBridge.executeInMainWorld({
        func: () => crypto.randomUUID()
      });
      invokeExtension(extensionId, "runtime.connectNative", {}, connectionId, application);
      const onMessage = (_event, message) => {
        receive(message);
      };
      import_electron2.ipcRenderer.on(`crx-native-msg-${connectionId}`, onMessage);
      import_electron2.ipcRenderer.once(`crx-native-msg-${connectionId}-disconnect`, () => {
        import_electron2.ipcRenderer.off(`crx-native-msg-${connectionId}`, onMessage);
        disconnect();
      });
      const send = (message) => {
        import_electron2.ipcRenderer.send(`crx-native-msg-${connectionId}`, message);
      };
      callback(connectionId, send);
    };
    const disconnectNative = (extensionId, connectionId) => {
      invokeExtension(extensionId, "runtime.disconnectNative", {}, connectionId);
    };
    const connectWebSocket = (extensionId, connectionId, url, protocols, callbacks) => {
      const onOpen = (_e, payload) => callbacks.onOpen(payload);
      const onMessage = (_e, payload) => callbacks.onMessage(payload);
      const onClose = (_e, payload) => callbacks.onClose(payload);
      const onError = () => callbacks.onError();
      import_electron2.ipcRenderer.on(`crx-websocket-open-${connectionId}`, onOpen);
      import_electron2.ipcRenderer.on(`crx-websocket-message-${connectionId}`, onMessage);
      import_electron2.ipcRenderer.on(`crx-websocket-close-${connectionId}`, onClose);
      import_electron2.ipcRenderer.on(`crx-websocket-error-${connectionId}`, onError);
      invokeExtension(extensionId, "websocket.connect", {}, connectionId, url, protocols);
      return () => {
        import_electron2.ipcRenderer.off(`crx-websocket-open-${connectionId}`, onOpen);
        import_electron2.ipcRenderer.off(`crx-websocket-message-${connectionId}`, onMessage);
        import_electron2.ipcRenderer.off(`crx-websocket-close-${connectionId}`, onClose);
        import_electron2.ipcRenderer.off(`crx-websocket-error-${connectionId}`, onError);
      };
    };
    const sendWebSocketFrame = (connectionId, data) => {
      import_electron2.ipcRenderer.send(`crx-websocket-send-${connectionId}`, data);
    };
    const closeWebSocket = (extensionId, connectionId, code, reason) => {
      invokeExtension(extensionId, "websocket.close", {}, connectionId, code, reason);
    };
    const electronContext = {
      invokeExtension,
      addExtensionListener,
      removeExtensionListener,
      hasExtensionListeners,
      connectNative,
      disconnectNative,
      connectWebSocket,
      sendWebSocketFrame,
      closeWebSocket
    };
    function mainWorldScript() {
      const electron = globalThis.electron || electronContext;
      const chrome = globalThis.chrome || {};
      const extensionId = chrome.runtime?.id;
      const manifest = extensionId && chrome.runtime.getManifest?.() || {};
      const invokeExtension2 = (fnName, opts = {}) => (...args) => electron.invokeExtension(extensionId, fnName, opts, ...args);
      function imageData2base64(imageData) {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");
        if (!ctx) return null;
        canvas.width = imageData.width;
        canvas.height = imageData.height;
        ctx.putImageData(imageData, 0, 0);
        return canvas.toDataURL();
      }
      const localStub = (result) => (...args) => {
        const callback = typeof args[args.length - 1] === "function" ? args.pop() : void 0;
        if (callback) {
          queueMicrotask(() => callback(result));
        }
        return Promise.resolve(result);
      };
      class ExtensionEvent {
        constructor(name) {
          this.name = name;
        }
        addListener(callback) {
          electron.addExtensionListener(extensionId, this.name, callback);
        }
        removeListener(callback) {
          electron.removeExtensionListener(extensionId, this.name, callback);
        }
        getRules(ruleIdentifiers, callback) {
          const cb = typeof ruleIdentifiers === "function" ? ruleIdentifiers : callback;
          if (cb) cb([]);
        }
        // Approximates whether *any* listener is registered for this event —
        // not specifically `callback`, since callback identity isn't tracked
        // across the IPC boundary (see hasExtensionListeners in event.ts).
        // Good enough for the common "have I already registered?" guard, and
        // — unlike throwing — doesn't crash the caller that relies on it.
        hasListener(callback) {
          return electron.hasExtensionListeners(this.name);
        }
        removeRules(ruleIdentifiers, callback) {
          const cb = typeof ruleIdentifiers === "function" ? ruleIdentifiers : callback;
          if (cb) cb();
        }
        addRules(rules, callback) {
          if (callback) callback([]);
        }
        hasListeners() {
          return electron.hasExtensionListeners(this.name);
        }
      }
      class ChromeSetting {
        constructor() {
          this.onChange = {
            addListener: () => {
            }
          };
        }
        set() {
        }
        get() {
        }
        clear() {
        }
      }
      class Event {
        constructor() {
          this.listeners = [];
        }
        _emit(...args) {
          this.listeners.forEach((listener) => {
            listener(...args);
          });
        }
        addListener(callback) {
          this.listeners.push(callback);
        }
        removeListener(callback) {
          const index = this.listeners.indexOf(callback);
          if (index > -1) {
            this.listeners.splice(index, 1);
          }
        }
      }
      class NativePort {
        constructor() {
          this.connectionId = "";
          this.connected = false;
          this.pending = [];
          this.name = "";
          this._init = (connectionId, send) => {
            this.connected = true;
            this.connectionId = connectionId;
            this._send = send;
            this.pending.forEach((msg) => this.postMessage(msg));
            this.pending = [];
            Object.defineProperty(this, "_init", { value: void 0 });
          };
          this.onMessage = new Event();
          this.onDisconnect = new Event();
        }
        _send(message) {
          this.pending.push(message);
        }
        _receive(message) {
          ;
          this.onMessage._emit(message);
        }
        _disconnect() {
          this.disconnect();
        }
        postMessage(message) {
          this._send(message);
        }
        disconnect() {
          if (this.connected) {
            electron.disconnectNative(extensionId, this.connectionId);
            this.onDisconnect._emit();
            this.connected = false;
          }
        }
      }
      const NativeEvent = globalThis.Event;
      class ProxyWebSocket extends EventTarget {
        constructor(url, protocols) {
          super();
          this.CONNECTING = 0;
          this.OPEN = 1;
          this.CLOSING = 2;
          this.CLOSED = 3;
          this.readyState = 0;
          this.bufferedAmount = 0;
          this.extensions = "";
          this.protocol = "";
          this.binaryType = "blob";
          this.onopen = null;
          this.onmessage = null;
          this.onerror = null;
          this.onclose = null;
          const resolved = new URL(url, self.location?.href);
          if (resolved.protocol !== "ws:" && resolved.protocol !== "wss:") {
            throw new SyntaxError(
              `Failed to construct 'WebSocket': The URL's scheme must be either 'ws' or 'wss'. '${resolved.protocol}' is not allowed.`
            );
          }
          this.url = resolved.href;
          this._id = crypto.randomUUID();
          const protocolList = typeof protocols === "string" ? [protocols] : protocols || [];
          this._dispose = electron.connectWebSocket(extensionId, this._id, this.url, protocolList, {
            onOpen: (payload) => {
              this.readyState = 1;
              this.protocol = payload.protocol || "";
              this.extensions = payload.extensions || "";
              this._fire("open", new NativeEvent("open"));
            },
            onMessage: (payload) => {
              let data;
              if (payload.binary) {
                const bytes = payload.data;
                const ab = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
                data = this.binaryType === "arraybuffer" ? ab : new Blob([ab]);
              } else {
                data = payload.data;
              }
              this._fire("message", new MessageEvent("message", { data }));
            },
            onClose: (payload) => {
              this.readyState = 3;
              this._teardown();
              this._fire(
                "close",
                new CloseEvent("close", {
                  code: payload.code,
                  reason: payload.reason,
                  wasClean: payload.wasClean
                })
              );
            },
            onError: () => {
              this._fire("error", new NativeEvent("error"));
            }
          });
        }
        send(data) {
          if (this.readyState === 0) {
            throw new DOMException(
              "Failed to execute 'send' on 'WebSocket': Still in CONNECTING state.",
              "InvalidStateError"
            );
          }
          if (this.readyState !== 1) return;
          if (typeof data === "string") {
            electron.sendWebSocketFrame(this._id, data);
          } else if (data instanceof Blob) {
            data.arrayBuffer().then((ab) => {
              if (this.readyState === 1) electron.sendWebSocketFrame(this._id, new Uint8Array(ab));
            });
          } else if (data instanceof ArrayBuffer) {
            electron.sendWebSocketFrame(this._id, new Uint8Array(data));
          } else if (ArrayBuffer.isView(data)) {
            electron.sendWebSocketFrame(
              this._id,
              new Uint8Array(data.buffer, data.byteOffset, data.byteLength)
            );
          } else {
            electron.sendWebSocketFrame(this._id, String(data));
          }
        }
        close(code, reason) {
          if (this.readyState === 2 || this.readyState === 3) return;
          this.readyState = 2;
          electron.closeWebSocket(extensionId, this._id, code, reason);
        }
        _teardown() {
          if (this._dispose) {
            this._dispose();
            this._dispose = void 0;
          }
        }
        _fire(type, event) {
          const handler = this["on" + type];
          if (typeof handler === "function") {
            try {
              handler.call(this, event);
            } catch (error) {
              console.error(error);
            }
          }
          this.dispatchEvent(event);
        }
      }
      ProxyWebSocket.CONNECTING = 0;
      ProxyWebSocket.OPEN = 1;
      ProxyWebSocket.CLOSING = 2;
      ProxyWebSocket.CLOSED = 3;
      const activeTabCache = {
        // Flat set of every active tab id, for O(1) lookups in patchSenderTab.
        ids: /* @__PURE__ */ new Set(),
        // windowId -> tabId, so activation in one window doesn't clear another's.
        byWindow: /* @__PURE__ */ new Map(),
        initialized: false,
        // Lazily initialized on first onMessage/onConnect subscription to avoid
        // registering tab listeners for extensions that never receive messages.
        init() {
          if (this.initialized) return;
          this.initialized = true;
          const setActive = (windowId, tabId) => {
            const previous = this.byWindow.get(windowId);
            if (typeof previous === "number") this.ids.delete(previous);
            this.byWindow.set(windowId, tabId);
            this.ids.add(tabId);
          };
          try {
            electron.addExtensionListener(
              extensionId,
              "tabs.onActivated",
              (info) => {
                setActive(info.windowId, info.tabId);
              }
            );
            electron.invokeExtension(extensionId, "tabs.query", {}, { active: true }).then(
              (tabs) => {
                tabs?.forEach((tab) => {
                  if (typeof tab.id === "number") setActive(tab.windowId, tab.id);
                });
              },
              () => {
              }
            );
          } catch {
          }
        }
      };
      const browserActionFactory = (base) => {
        const api = {
          ...base,
          setTitle: invokeExtension2("browserAction.setTitle"),
          getTitle: invokeExtension2("browserAction.getTitle"),
          setIcon: invokeExtension2("browserAction.setIcon", {
            serialize: (details) => {
              if (details.imageData) {
                if (manifest.manifest_version === 3) {
                  console.warn(
                    "action.setIcon with imageData is not yet supported by electron-chrome-extensions"
                  );
                  details.imageData = void 0;
                } else if (details.imageData instanceof ImageData) {
                  details.imageData = imageData2base64(details.imageData);
                } else {
                  details.imageData = Object.entries(details.imageData).reduce(
                    (obj, pair) => {
                      obj[pair[0]] = imageData2base64(pair[1]);
                      return obj;
                    },
                    {}
                  );
                }
              }
              return [details];
            }
          }),
          setPopup: invokeExtension2("browserAction.setPopup"),
          getPopup: invokeExtension2("browserAction.getPopup"),
          setBadgeText: invokeExtension2("browserAction.setBadgeText"),
          getBadgeText: invokeExtension2("browserAction.getBadgeText"),
          setBadgeBackgroundColor: invokeExtension2("browserAction.setBadgeBackgroundColor"),
          getBadgeBackgroundColor: invokeExtension2("browserAction.getBadgeBackgroundColor"),
          getUserSettings: invokeExtension2("browserAction.getUserSettings"),
          enable: invokeExtension2("browserAction.enable", { noop: true }),
          disable: invokeExtension2("browserAction.disable", { noop: true }),
          openPopup: invokeExtension2("browserAction.openPopup"),
          onClicked: new ExtensionEvent("browserAction.onClicked")
        };
        return api;
      };
      const apiDefinitions = {
        action: {
          shouldInject: () => manifest.manifest_version === 3 && !!manifest.action,
          factory: browserActionFactory
        },
        alarms: {
          factory: (base) => {
            return {
              ...base,
              create: invokeExtension2("alarms.create"),
              get: invokeExtension2("alarms.get"),
              getAll: invokeExtension2("alarms.getAll"),
              clear: invokeExtension2("alarms.clear"),
              clearAll: invokeExtension2("alarms.clearAll"),
              onAlarm: new ExtensionEvent("alarms.onAlarm")
            };
          }
        },
        browserAction: {
          shouldInject: () => manifest.manifest_version === 2 && !!manifest.browser_action,
          factory: browserActionFactory
        },
        commands: {
          factory: (base) => {
            return {
              ...base,
              getAll: invokeExtension2("commands.getAll"),
              onCommand: new ExtensionEvent("commands.onCommand")
            };
          }
        },
        contextMenus: {
          factory: (base) => {
            let menuCounter = 0;
            const menuCallbacks = {};
            const menuCreate = invokeExtension2("contextMenus.create");
            let hasInternalListener = false;
            const addInternalListener = () => {
              api.onClicked.addListener((info, tab) => {
                const callback = menuCallbacks[info.menuItemId];
                if (callback && tab) callback(info, tab);
              });
              hasInternalListener = true;
            };
            const menuUpdate = invokeExtension2("contextMenus.update");
            const api = {
              ...base,
              create: function(createProperties, callback) {
                if (typeof createProperties.id === "undefined") {
                  createProperties.id = `${++menuCounter}`;
                }
                if (createProperties.onclick) {
                  if (!hasInternalListener) addInternalListener();
                  menuCallbacks[createProperties.id] = createProperties.onclick;
                  delete createProperties.onclick;
                }
                menuCreate(createProperties, callback);
                return createProperties.id;
              },
              update: function(id, updateProperties, callback) {
                if (updateProperties.onclick) {
                  if (!hasInternalListener) addInternalListener();
                  menuCallbacks[id] = updateProperties.onclick;
                  delete updateProperties.onclick;
                }
                return menuUpdate(id, updateProperties, callback);
              },
              remove: invokeExtension2("contextMenus.remove"),
              removeAll: invokeExtension2("contextMenus.removeAll"),
              onClicked: new ExtensionEvent("contextMenus.onClicked")
            };
            return api;
          }
        },
        cookies: {
          factory: (base) => {
            return {
              ...base,
              get: invokeExtension2("cookies.get"),
              getAll: invokeExtension2("cookies.getAll"),
              set: invokeExtension2("cookies.set"),
              remove: invokeExtension2("cookies.remove"),
              getAllCookieStores: invokeExtension2("cookies.getAllCookieStores"),
              onChanged: new ExtensionEvent("cookies.onChanged")
            };
          }
        },
        downloads: {
          factory: (base) => {
            return {
              ...base,
              acceptDanger: invokeExtension2("downloads.acceptDanger", { noop: true }),
              cancel: invokeExtension2("downloads.cancel"),
              download: invokeExtension2("downloads.download"),
              erase: invokeExtension2("downloads.erase"),
              getFileIcon: invokeExtension2("downloads.getFileIcon", { noop: true }),
              open: invokeExtension2("downloads.open"),
              pause: invokeExtension2("downloads.pause"),
              removeFile: invokeExtension2("downloads.removeFile", { noop: true }),
              resume: invokeExtension2("downloads.resume"),
              search: invokeExtension2("downloads.search"),
              setUiOptions: invokeExtension2("downloads.setUiOptions", { noop: true }),
              show: invokeExtension2("downloads.show"),
              showDefaultFolder: invokeExtension2("downloads.showDefaultFolder"),
              onChanged: new ExtensionEvent("downloads.onChanged"),
              onCreated: new ExtensionEvent("downloads.onCreated"),
              onDeterminingFilename: new ExtensionEvent("downloads.onDeterminingFilename"),
              onErased: new ExtensionEvent("downloads.onErased")
            };
          }
        },
        extension: {
          factory: (base) => {
            return {
              ...base,
              isAllowedFileSchemeAccess: invokeExtension2("extension.isAllowedFileSchemeAccess", {
                noop: true,
                defaultResponse: false
              }),
              isAllowedIncognitoAccess: invokeExtension2("extension.isAllowedIncognitoAccess", {
                noop: true,
                defaultResponse: false
              }),
              // TODO: Add native implementation
              getViews: () => []
            };
          }
        },
        // Font settings aren't configurable in Electron, so this is a local
        // stub which reports Chrome's default values as 'not_controllable'.
        fontSettings: {
          factory: (base) => {
            const genericFontList = [
              { fontId: "sans-serif", displayName: "Sans-Serif" },
              { fontId: "serif", displayName: "Serif" },
              { fontId: "monospace", displayName: "Monospace" }
            ];
            return {
              ...base,
              clearDefaultFixedFontSize: localStub(),
              clearDefaultFontSize: localStub(),
              clearFont: localStub(),
              clearMinimumFontSize: localStub(),
              getDefaultFixedFontSize: localStub({
                pixelSize: 13,
                levelOfControl: "not_controllable"
              }),
              getDefaultFontSize: localStub({ pixelSize: 16, levelOfControl: "not_controllable" }),
              getFont: localStub({ fontId: "", levelOfControl: "not_controllable" }),
              getFontList: localStub(genericFontList),
              getMinimumFontSize: localStub({ pixelSize: 0, levelOfControl: "not_controllable" }),
              setDefaultFixedFontSize: localStub(),
              setDefaultFontSize: localStub(),
              setFont: localStub(),
              setMinimumFontSize: localStub(),
              onDefaultFixedFontSizeChanged: new Event(),
              onDefaultFontSizeChanged: new Event(),
              onFontChanged: new Event(),
              onMinimumFontSizeChanged: new Event()
            };
          }
        },
        identity: {
          shouldInject: () => !!manifest.permissions?.includes("identity"),
          factory: (base) => {
            return {
              ...base,
              getRedirectURL: (path) => {
                const cleanPath = (path || "").replace(/^\//, "");
                return `https://${extensionId}.chromiumapp.org/${cleanPath}`;
              },
              launchWebAuthFlow: invokeExtension2("identity.launchWebAuthFlow")
            };
          }
        },
        i18n: {
          shouldInject: () => manifest.manifest_version === 3,
          factory: (base) => {
            if (base.getMessage) {
              return base;
            }
            return {
              ...base,
              getUILanguage: () => "en-US",
              getAcceptLanguages: (callback) => {
                const results = ["en-US"];
                if (callback) {
                  queueMicrotask(() => callback(results));
                }
                return Promise.resolve(results);
              },
              getMessage: (messageName) => messageName
            };
          }
        },
        idle: {
          factory: (base) => {
            return {
              ...base,
              queryState: invokeExtension2("idle.queryState"),
              setDetectionInterval: invokeExtension2("idle.setDetectionInterval"),
              getAutoLockDelay: invokeExtension2("idle.getAutoLockDelay"),
              onStateChanged: new ExtensionEvent("idle.onStateChanged")
            };
          }
        },
        management: {
          factory: (base) => {
            return {
              ...base,
              // Electron's native implementation of getAll never resolves.
              get: invokeExtension2("management.get"),
              getAll: invokeExtension2("management.getAll"),
              getSelf: invokeExtension2("management.getSelf")
            };
          }
        },
        notifications: {
          factory: (base) => {
            return {
              ...base,
              clear: invokeExtension2("notifications.clear"),
              create: invokeExtension2("notifications.create"),
              getAll: invokeExtension2("notifications.getAll"),
              getPermissionLevel: invokeExtension2("notifications.getPermissionLevel"),
              update: invokeExtension2("notifications.update"),
              onClicked: new ExtensionEvent("notifications.onClicked"),
              onButtonClicked: new ExtensionEvent("notifications.onButtonClicked"),
              onClosed: new ExtensionEvent("notifications.onClosed")
            };
          }
        },
        offscreen: {
          shouldInject: () => manifest.manifest_version === 3,
          factory: (base) => {
            return {
              ...base,
              createDocument: invokeExtension2("offscreen.createDocument"),
              closeDocument: invokeExtension2("offscreen.closeDocument"),
              hasDocument: invokeExtension2("offscreen.hasDocument"),
              // Enum normally provided by Chrome; extensions reference it when
              // building createDocument() parameters.
              Reason: {
                TESTING: "TESTING",
                AUDIO_PLAYBACK: "AUDIO_PLAYBACK",
                IFRAME_SCRIPTING: "IFRAME_SCRIPTING",
                DOM_SCRAPING: "DOM_SCRAPING",
                BLOBS: "BLOBS",
                DOM_PARSER: "DOM_PARSER",
                USER_MEDIA: "USER_MEDIA",
                DISPLAY_MEDIA: "DISPLAY_MEDIA",
                WEB_RTC: "WEB_RTC",
                CLIPBOARD: "CLIPBOARD",
                LOCAL_STORAGE: "LOCAL_STORAGE",
                WORKERS: "WORKERS",
                BATTERY_STATUS: "BATTERY_STATUS",
                MATCH_MEDIA: "MATCH_MEDIA",
                GEOLOCATION: "GEOLOCATION"
              }
            };
          }
        },
        permissions: {
          factory: (base) => {
            return {
              ...base,
              contains: invokeExtension2("permissions.contains"),
              getAll: invokeExtension2("permissions.getAll"),
              remove: invokeExtension2("permissions.remove"),
              request: invokeExtension2("permissions.request"),
              onAdded: new ExtensionEvent("permissions.onAdded"),
              onRemoved: new ExtensionEvent("permissions.onRemoved")
            };
          }
        },
        privacy: {
          factory: (base) => {
            return {
              ...base,
              network: {
                networkPredictionEnabled: new ChromeSetting(),
                webRTCIPHandlingPolicy: new ChromeSetting()
              },
              services: {
                autofillAddressEnabled: new ChromeSetting(),
                autofillCreditCardEnabled: new ChromeSetting(),
                passwordSavingEnabled: new ChromeSetting()
              },
              websites: {
                hyperlinkAuditingEnabled: new ChromeSetting()
              }
            };
          }
        },
        runtime: {
          factory: (base) => {
            const patchSenderTab = (sender) => {
              const tab = sender?.tab;
              if (tab && typeof tab.id === "number" && activeTabCache.ids.has(tab.id)) {
                tab.active = true;
                tab.highlighted = true;
              }
              return sender;
            };
            const wrapSenderEvent = (event, transform) => {
              if (!event?.addListener) return event;
              const wrappers = /* @__PURE__ */ new WeakMap();
              const originalAdd = event.addListener.bind(event);
              const originalRemove = event.removeListener.bind(event);
              return Object.assign(Object.create(event), {
                addListener: (callback, ...rest) => {
                  activeTabCache.init();
                  const wrapped = (...args) => {
                    transform(args);
                    return callback(...args);
                  };
                  wrappers.set(callback, wrapped);
                  return originalAdd(wrapped, ...rest);
                },
                removeListener: (callback) => {
                  originalRemove(wrappers.get(callback) ?? callback);
                  wrappers.delete(callback);
                },
                hasListener: (callback) => event.hasListener?.(wrappers.get(callback) ?? callback)
              });
            };
            return {
              ...base,
              // onMessage signature is (message, sender, sendResponse) — patch arg[1].
              onMessage: wrapSenderEvent(base.onMessage, (args) => {
                patchSenderTab(args[1]);
              }),
              // onConnect signature is (port) — the sender lives on the port.
              onConnect: wrapSenderEvent(base.onConnect, (args) => {
                patchSenderTab(args[0]?.sender);
              }),
              connectNative: (application) => {
                const port = new NativePort();
                const receive = port._receive.bind(port);
                const disconnect = port._disconnect.bind(port);
                const callback = (connectionId, send) => {
                  port._init(connectionId, send);
                };
                electron.connectNative(extensionId, application, receive, disconnect, callback);
                return port;
              },
              openOptionsPage: invokeExtension2("runtime.openOptionsPage"),
              sendNativeMessage: invokeExtension2("runtime.sendNativeMessage")
            };
          }
        },
        // There's no side panel UI in Electron; stubbed so MV3 extensions that
        // probe for the API don't crash.
        sidePanel: {
          shouldInject: () => manifest.manifest_version === 3,
          factory: (base) => {
            return {
              ...base,
              getOptions: localStub({ enabled: false }),
              setOptions: localStub(),
              getPanelBehavior: localStub({ openPanelOnActionClick: false }),
              setPanelBehavior: localStub(),
              open: localStub()
            };
          }
        },
        storage: {
          factory: (base) => {
            const local = base && base.local;
            return {
              ...base,
              // TODO: provide a backend for browsers to opt-in to
              managed: local,
              sync: local
            };
          }
        },
        tabs: {
          factory: (base) => {
            const api = {
              ...base,
              create: invokeExtension2("tabs.create"),
              executeScript: async function(arg1, arg2, arg3) {
                if (typeof arg1 === "object") {
                  const [activeTab] = await api.query({
                    active: true,
                    windowId: chrome.windows.WINDOW_ID_CURRENT
                  });
                  return api.executeScript(activeTab.id, arg1, arg2);
                } else {
                  return base.executeScript(
                    arg1,
                    arg2,
                    arg3
                  );
                }
              },
              captureVisibleTab: invokeExtension2("tabs.captureVisibleTab"),
              get: invokeExtension2("tabs.get"),
              getCurrent: invokeExtension2("tabs.getCurrent"),
              getAllInWindow: invokeExtension2("tabs.getAllInWindow"),
              insertCSS: invokeExtension2("tabs.insertCSS"),
              query: invokeExtension2("tabs.query"),
              reload: invokeExtension2("tabs.reload"),
              update: invokeExtension2("tabs.update"),
              remove: invokeExtension2("tabs.remove"),
              goBack: invokeExtension2("tabs.goBack"),
              goForward: invokeExtension2("tabs.goForward"),
              onCreated: new ExtensionEvent("tabs.onCreated"),
              onRemoved: new ExtensionEvent("tabs.onRemoved"),
              onUpdated: new ExtensionEvent("tabs.onUpdated"),
              onActivated: new ExtensionEvent("tabs.onActivated"),
              onReplaced: new ExtensionEvent("tabs.onReplaced")
            };
            return api;
          }
        },
        topSites: {
          factory: () => {
            return {
              get: invokeExtension2("topSites.get", { noop: true, defaultResponse: [] })
            };
          }
        },
        webNavigation: {
          factory: (base) => {
            return {
              ...base,
              getFrame: invokeExtension2("webNavigation.getFrame"),
              getAllFrames: invokeExtension2("webNavigation.getAllFrames"),
              onBeforeNavigate: new ExtensionEvent("webNavigation.onBeforeNavigate"),
              onCommitted: new ExtensionEvent("webNavigation.onCommitted"),
              onCompleted: new ExtensionEvent("webNavigation.onCompleted"),
              onCreatedNavigationTarget: new ExtensionEvent(
                "webNavigation.onCreatedNavigationTarget"
              ),
              onDOMContentLoaded: new ExtensionEvent("webNavigation.onDOMContentLoaded"),
              onErrorOccurred: new ExtensionEvent("webNavigation.onErrorOccurred"),
              onHistoryStateUpdated: new ExtensionEvent("webNavigation.onHistoryStateUpdated"),
              onReferenceFragmentUpdated: new ExtensionEvent(
                "webNavigation.onReferenceFragmentUpdated"
              ),
              onTabReplaced: new ExtensionEvent("webNavigation.onTabReplaced")
            };
          }
        },
        windows: {
          factory: (base) => {
            return {
              ...base,
              WINDOW_ID_NONE: -1,
              WINDOW_ID_CURRENT: -2,
              get: invokeExtension2("windows.get"),
              getCurrent: invokeExtension2("windows.getCurrent"),
              getLastFocused: invokeExtension2("windows.getLastFocused"),
              getAll: invokeExtension2("windows.getAll"),
              create: invokeExtension2("windows.create"),
              update: invokeExtension2("windows.update"),
              remove: invokeExtension2("windows.remove"),
              onCreated: new ExtensionEvent("windows.onCreated"),
              onRemoved: new ExtensionEvent("windows.onRemoved"),
              onFocusChanged: new ExtensionEvent("windows.onFocusChanged"),
              onBoundsChanged: new ExtensionEvent("windows.onBoundsChanged")
            };
          }
        }
      };
      Object.keys(apiDefinitions).forEach((key) => {
        const apiName = key;
        const baseApi = chrome[apiName];
        const api = apiDefinitions[apiName];
        if (api.shouldInject && !api.shouldInject()) return;
        Object.defineProperty(chrome, apiName, {
          value: api.factory(baseApi),
          enumerable: true,
          configurable: true
        });
      });
      if (extensionId) activeTabCache.init();
      const isServiceWorker = typeof self.ServiceWorkerGlobalScope !== "undefined" && self instanceof self.ServiceWorkerGlobalScope;
      if (isServiceWorker && extensionId) {
        try {
          ;
          globalThis.WebSocket = ProxyWebSocket;
        } catch (error) {
          console.error("Failed to install WebSocket proxy", error);
        }
      }
      const nativeWebRequest = chrome.webRequest;
      if (nativeWebRequest) {
        const webRequestEventNames = [
          "onBeforeRequest",
          "onBeforeSendHeaders",
          "onSendHeaders",
          "onHeadersReceived",
          "onAuthRequired",
          "onResponseStarted",
          "onBeforeRedirect",
          "onCompleted",
          "onErrorOccurred",
          "onActionIgnored"
        ];
        const missing = webRequestEventNames.filter((name) => nativeWebRequest[name] === void 0);
        if (missing.length > 0) {
          const makeInertEvent = () => ({
            addListener: () => {
            },
            removeListener: () => {
            },
            hasListener: () => false,
            hasListeners: () => false,
            getRules: (...args) => {
              const cb = args[args.length - 1];
              if (typeof cb === "function") cb([]);
            },
            addRules: () => {
            },
            removeRules: (...args) => {
              const cb = args[args.length - 1];
              if (typeof cb === "function") cb();
            }
          });
          try {
            const repaired = { ...nativeWebRequest };
            for (const name of missing) repaired[name] = makeInertEvent();
            Object.defineProperty(chrome, "webRequest", {
              value: repaired,
              enumerable: true,
              configurable: true
            });
          } catch (error) {
            console.error("Failed to repair chrome.webRequest events", error);
          }
        }
      }
      if (isServiceWorker && extensionId) {
        const dnr = chrome.declarativeNetRequest;
        const rulesets = manifest.declarative_net_request?.rule_resources || [];
        const enabledIds = rulesets.filter((r) => r && r.enabled !== false && typeof r.id === "string").map((r) => r.id);
        if (dnr?.updateEnabledRulesets && enabledIds.length > 0) {
          try {
            Promise.resolve(dnr.updateEnabledRulesets({ enableRulesetIds: enabledIds })).catch(
              (error) => console.error("Failed to enable static DNR rulesets", error)
            );
          } catch (error) {
            console.error("Failed to enable static DNR rulesets", error);
          }
        }
      }
      delete globalThis.electron;
      Object.freeze(chrome);
    }
    if (!process.contextIsolated) {
      console.warn(`injectExtensionAPIs: context isolation disabled in ${location.href}`);
      mainWorldScript();
      return;
    }
    try {
      import_electron2.contextBridge.exposeInMainWorld("electron", electronContext);
      if ("executeInMainWorld" in import_electron2.contextBridge) {
        ;
        import_electron2.contextBridge.executeInMainWorld({
          func: mainWorldScript
        });
      } else {
        import_electron2.webFrame.executeJavaScript(`(${mainWorldScript}());`);
      }
    } catch (error) {
      console.error(`injectExtensionAPIs error (${location.href})`);
      console.error(error);
    }
  };

  // src/preload.ts
  if (process.type === "service-worker" || location.href.startsWith("chrome-extension://")) {
    injectExtensionAPIs();
  }
})();
