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
import { useState, useEffect } from "react";

const overlayStyle = css`
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  opacity: 1;
  animation: fadeIn 0.2s forwards;
  padding: 10px;
  flex-direction: column;
  box-sizing: border-box;

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes fadeOut {
    from {
      opacity: 1;
    }
    to {
      opacity: 0;
    }
  }
`;

function Overlay({ open, children }) {
  const [isVisible, setIsVisible] = useState(open);
  const [animation, setAnimation] = useState(open ? "fadeIn" : "fadeOut");

  useEffect(() => {
    if (open) {
      setIsVisible(true);
      setAnimation("fadeIn");
    } else {
      setAnimation("fadeOut");
      const timer = setTimeout(() => setIsVisible(false), 200); // Match the duration of the fade-out animation
      return () => clearTimeout(timer);
    }
  }, [open]);

  return (
    <div style={{ ...!isVisible && { display: 'none' } }} css={[overlayStyle, !open ? css`pointer-events: none;` : css``, css`animation: ${animation} 0.5s forwards;`]}>
      {children}
    </div>
  );
}

export default Overlay;