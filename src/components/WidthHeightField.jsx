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
import { InputNumber } from 'antd';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faTimes } from '@fortawesome/free-solid-svg-icons';

const WidthHeightField = ({ value = [], onChange }) => {
  const [width, height] = value;

  const triggerChange = (changedValue) => {
    if (onChange) {
      onChange([changedValue.width || width, changedValue.height || height]);
    }
  };

  const onWidthChange = (newWidth) => {
    triggerChange({ width: newWidth });
  };

  const onHeightChange = (newHeight) => {
    triggerChange({ height: newHeight });
  };

  const styleCol2 = css`display: flex; align-items: center;`;

  return (
    <div>
      <div css={styleCol2}>
        <InputNumber
          value={width}
          onChange={onWidthChange}
          min={256}
          placeholder="Width"
          size="small"
        />
      </div>
      <div style={{ margin: '8px 0' }}>
        <FontAwesomeIcon icon={faTimes} />
      </div>
      <div css={styleCol2}>
        <InputNumber
          value={height}
          onChange={onHeightChange}
          min={256}
          placeholder="Height"
          size="small"
        />
      </div>
    </div>
  );
};

export default WidthHeightField;