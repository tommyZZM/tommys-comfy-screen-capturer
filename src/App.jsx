// Copyright (c) 2025 tommyZZM
// tommys-comfy-screen-capturer is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.
//

/** @jsxImportSource @emotion/react */
import { css } from "@emotion/react";
import { useState, useEffect, useMemo } from "react";
import { Modal, InputNumber, Button, Form, Input, message, Switch, Spin } from "antd";
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faThumbtack, faThumbtackSlash, faCamera, faSave, faCheck, faCircleXmark, faGears, faServer, faArrowsAlt, faXmark } from '@fortawesome/free-solid-svg-icons';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import { writeText, readText } from "tauri-plugin-clipboard-api";
import packageJson from '../src-tauri/tauri.conf.json';
import * as appLess from "./App.module.less";
import Overlay from './Overlay';
import WidthHeightField from "./components/WidthHeightField";
import classNames from 'classnames/bind';

const appClassNames = classNames.bind(appLess);

const BASE_WINDOW_TITLE = packageJson.app.windows[0].title;

const DEFAULT_PORT = 12666;

function getWindowRandomId() {
  return Math.random().toString(36).substring(2, 9);
}

function getUrlCaptureScreen(port) {
  return `http://127.0.0.1:${port}/capture_screen`;
}

function App() {
  const [changed, setChanged] = useState(0);
  const [isPendingInitial, setIsPendingInitial] = useState(true);
  const [windowId, setWindowId] = useState(getWindowRandomId());
  const [windowSize, setWindowSize] = useState({
    width: window.innerWidth,
    height: window.innerHeight,
  });
  const [isResizeModalVisible, setIsResizeModalVisible] = useState(false);
  const [newWidth, setNewWidth] = useState(window.innerWidth);
  const [newHeight, setNewHeight] = useState(window.innerHeight);
  const [isPendingCaptureScreenByButton, setIsPendingCaptureScreenByButton] = useState(false);
  const [serverStarted, setServerStarted] = useState(false);

  const [isPin, setIsPin] = useState(false); // 添加 isPin 状态
  const [isPreviewImageVisible, setIsPreviewImageVisible] = useState(false);
  const [base64PreviewImage, setBase64PreviewImage] = useState(null);
  const [form] = Form.useForm();

  const { serverPort, isStartServer } = form.getFieldsValue();

  useEffect(() => {
    setIsPendingInitial(true);

    function handleResize() {
      setWindowSize({
        width: window.innerWidth,
        height: window.innerHeight,
      });
      form.setFieldsValue({ size: [window.innerWidth, window.innerHeight] });
    }

    window.addEventListener("resize", handleResize);

    const pending = (async () => {
      const unlisten_is_pin_changed = await listen('is_pin_changed', async (event) => {
        setIsPin(event.payload);
      });

      await invoke('set_is_pin', { isPin: false });

      const unliste_copy_url_to_clipboard = await listen('copy_screenshot_url', () => {
        handleCopyUrlToClipboard();
      });

      const isStartServerBackend = await invoke('get_is_server_running');
      form.setFieldValue('isStartServer', isStartServerBackend);
      setServerStarted(isStartServerBackend);

      setIsPendingInitial(false);

      return () => {
        unlisten_is_pin_changed();
        unliste_copy_url_to_clipboard();
      }
    })();

    return () => {
      window.removeEventListener("resize", handleResize);
      pending.then(cleanup => cleanup());
    }
  }, []);

  useEffect(() => {
    (async () => {
      await invoke("set_window_title", { title: BASE_WINDOW_TITLE + `[${windowId}]` });
    })();
  }, [windowId]);

  useEffect(() => {
    (async () => {
      if (isPendingInitial) {
        return;
      }
      // console.log('serverStarted?', serverStarted, serverPort);
      if (serverStarted && serverPort) {
        await invoke("restart_http_server", { port: serverPort, scaleFactor: window.devicePixelRatio });
      } else {
        await invoke("stop_http_server");
      }
    })();
  }, [serverStarted, serverPort, isPendingInitial]);

  useEffect(() => {
    (() => {
      if (isPendingInitial) {
        return;
      }
      setIsPreviewImageVisible(false);
    })();
  }, [isPin]);

  const showResizeModal = () => {
    setIsResizeModalVisible(true);
  };

  const handleResizeModalOk = async () => {
    const values = await form.validateFields();
    setIsResizeModalVisible(false);
    const [width, height] = values.size;
    await invoke("resize_window", { width, height });

    // Update serverStarted based on isStartServer value
    setServerStarted(values.isStartServer);
  };

  const handleResizeModalCancel = () => {
    setIsResizeModalVisible(false);
  };

  const handleCaptureScreenByClick = async () => {
    setIsPendingCaptureScreenByButton(true);

    const encodedImage = await (async () => {
      try {
        const res = await invoke('capture_window_screenshot', { scaleFactor: window.devicePixelRatio });
        return res;
      } catch (error) {
        message.error(typeof error === "string" ? error : (error.message || "capture screen failed."));
        throw error;
      }
    })();

    setBase64PreviewImage(encodedImage);

    setIsPreviewImageVisible(true);

    setIsPendingCaptureScreenByButton(false);
  };

  const handleSavePreviewImage = async () => {
    try {
      const fileHandle = await window.showSaveFilePicker({
        suggestedName: `screenshot-${windowId}.png`,
        types: [{
          description: 'PNG Image',
          accept: { 'image/png': ['.png'] },
        }],
      });
      const writable = await fileHandle.createWritable();
      await writable.write(new Blob([Uint8Array.from(atob(base64PreviewImage), c => c.charCodeAt(0))], { type: 'image/png' }));
      await writable.close();

      setIsPreviewImageVisible(false);

      setWindowId(getWindowRandomId());
    } catch (error) {
      message.error("Failed to save screenshot.");
    } finally {
      // setBase64PreviewImage(null);
    }
  }

  const resetPreviewImage = () => {
    setIsPreviewImageVisible(false);
    // setBase64PreviewImage(null);
  };

  // 用于复制 URL 到剪贴板
  const handleCopyUrlToClipboard = () => {
    const url = getUrlCaptureScreen(serverPort);
    writeText(url).then(() => {
      message.success("URL copied!");
    }).catch(err => {
      console.error(err);
      message.error("Failed to copy URL.");
    });
  };

  // 用于切换 isPin 状态
  const togglePin = () => {
    const isPinNext = !isPin;
    invoke('set_is_pin', { isPin: isPinNext });
  };

  const windowToolbarStyle = css`
    position: fixed;
    bottom: 10px;
    right: 10px;
    left: 10px;
    font-size: 16px;
    padding: 5px;
    opacity: 0.9;
    display: flex;
    align-items: center;
  
    button {
      margin-left: 10px;
    }
  `;

  const flexAlignCenterStyle = css`display: flex; align-items: center;justify-content: center;`;

  const flexAlignStartStyle = css`display: flex; align-items: center;justify-content: flex-start;`;

  const flexFillRestStyle = css`flex-grow: 1;`;

  const panelStyle = css`
    background-color: #fff;
    flex-grow: 1;
    align-self: stretch;
    padding: 20px;
    border-radius: 5px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
  `;

  const panelPreviewStyle = css`
    background-color: transparent;
    flex-grow: 1;
    align-self: stretch;
    padding: 20px;
    border-radius: 5px;
    align-items: center;
    display: flex;
    flex-direction: column;
    justify-content: center;
    box-sizing: border-box;
    align-self: stretch;
    max-height: 100%;

    img {
      max-width: 100%;
      max-height: 100%;
      object-fit: contain;
      text-align: center;
      border-radius: 5px;
      overflow: hidden;
    }
  `;


  const serverStatusButton = css`
    color: ${serverStarted ? '#2ecc71' : '#95a5a6'};
    display: flex;
    align-items: center;
  `;

  const windowToolbarButtonText = css`
    @media (max-width: 360px) {
      display: none;
    }
  `;

  const windowTopbarStyle = css`
    position: absolute;
    top: 10px;
    right: 10px;
    z-index: 1000;
    display: flex;
    align-items: center;
  `;

  const urlCaptureScreen = useMemo(() => getUrlCaptureScreen(serverPort), [serverPort]);

  return (
    <main 
      className={appClassNames('app', { isPin })} 
      style={{ ...isPin && { opacity: 0.77 } }}
    >
      <Spin wrapperClassName={appLess.appSpinWrapper} spinning={isPendingInitial}>
        <div style={{ ...isPin && { opacity: 0 } }} css={windowTopbarStyle}>
          <Button style={{ marginRight: 10 }} {...isPin && { type: 'primary' }} size={'small'} onClick={togglePin}>
            <FontAwesomeIcon icon={isPin ? faThumbtack : faThumbtackSlash} />
          </Button>
          <Button size={'small'} onClick={() => invoke('quit_app')}>
            <FontAwesomeIcon icon={faXmark} />
          </Button>
        </div>
        <div style={{ ...isPin && { opacity: 0 } }} css={windowToolbarStyle}>
          <Button css={serverStatusButton} size={'small'} onClick={serverStarted ? () => openUrl(urlCaptureScreen) : showResizeModal}>
            <span css={windowToolbarButtonText}>Url</span>
            <FontAwesomeIcon icon={faServer} />
          </Button>
          <div css={flexFillRestStyle} />
          <Button size={'small'} onClick={handleCaptureScreenByClick} loading={isPendingCaptureScreenByButton}>
            <FontAwesomeIcon icon={faCamera} /><span css={windowToolbarButtonText}>Capture</span>
          </Button>
          <Button onClick={showResizeModal} type="primary" size="small">
            <FontAwesomeIcon icon={faGears} />{windowSize.width} x {windowSize.height}
          </Button>
        </div>
        <div className={appLess.appDraggableWrapper}>
          <div className={appLess.appDraggableHandle} onMouseDown={() => !isPin &&invoke('start_dragging')}><FontAwesomeIcon icon={faArrowsAlt} /></div>
        </div>
      </Spin>
      <Overlay open={isPreviewImageVisible}>
        <div css={panelPreviewStyle}>
          <div css={flexAlignCenterStyle} style={{ position: 'relative', flexGrow: 1, width: '100%' }}>
            <div style={{ position: 'absolute', top: 0, left: 0, width: '100%', height: '100%', textAlign: 'center' }}>
              <img src={`data:image/png;base64,${base64PreviewImage}`} alt="screenshot" />
            </div>
          </div>
          <div css={flexAlignCenterStyle} style={{ marginTop: 20 }}>
            <Button style={{ marginRight: 10 }} size={'small'} onClick={resetPreviewImage}>
              <FontAwesomeIcon icon={faCircleXmark} />
              <span>Cancel</span>
            </Button>
            <Button type="primary" size={'small'} onClick={handleSavePreviewImage}>
              <FontAwesomeIcon icon={faSave} />
              <span>Save</span>
            </Button>
          </div>
        </div>
      </Overlay>

      <Overlay open={isResizeModalVisible}>
        <div css={panelStyle}>
          <Form onChange={() => setChanged(a => a + 1)} form={form} initialValues={{ size: [newWidth, newHeight], isStartServer: serverStarted, serverPort: DEFAULT_PORT }}>
            <Form.Item label="Size" name="size" rules={[{ required: true, message: 'Please input size!' }]}>
              <WidthHeightField size="small" />
            </Form.Item>
            <div style={{ height: 32, paddingBottom: 8 }} css={flexAlignStartStyle}>Server {isStartServer && <div style={{ marginLeft: 5 }}><small><a onClick={() => handleCopyUrlToClipboard(urlCaptureScreen)}>{urlCaptureScreen}</a></small></div>}</div>
            <Form.Item label={null} name="isStartServer" valuePropName="checked">
              <Switch onChange={() => setChanged(a => a + 1)}>Start Server</Switch>
            </Form.Item>
            
            <Form.Item label="ServerPort" name="serverPort" rules={[{ required: true, message: 'Please input serverPort!' }]}>
              <InputNumber onChange={() => setChanged(a => a + 1)} min={1} max={65535} />
            </Form.Item>
          </Form>
          <div css={flexFillRestStyle} />
          <div style={{ fontSize: 12 }}>
            <div>About:</div>
            <p>Repository: <a href="https://github.com/tommyZZM/tommys-comfy-screen-capturer" target="_blank" rel="noopener noreferrer">https://github.com/tommyZZM/tommys-comfy-screen-capturer</a></p>
            <p>This software is licensed under the Mulan PSL v2. See the LICENSE file for details.</p>
          </div>
          <div css={flexAlignCenterStyle} style={{ marginTop: 20 }}>
            <Button style={{ marginRight: 10 }} size={'small'} onClick={handleResizeModalCancel}>
              <FontAwesomeIcon icon={faCircleXmark} />
              <span>Cancel</span>
            </Button>
            <Button type="primary" size={'small'} onClick={handleResizeModalOk}>
              <FontAwesomeIcon icon={faCheck} />
              <span>OK</span>
            </Button>
          </div>
        </div>
      </Overlay>
    </main>
  );
}

export default App;
