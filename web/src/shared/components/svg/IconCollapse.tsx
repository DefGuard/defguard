import * as React from 'react';
import type { SVGProps } from 'react';
const SvgIconCollapse = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={23} height={22} fill="none" {...props}>
    <path
      fill="#0C8CE0"
      d="M4.5 13v4a1 1 0 0 0 1 1h4a1 1 0 1 0 0-2H7.913l1.026-1.026a1 1 0 1 0-1.414-1.414L6.5 14.585V13a1 1 0 1 0-2 0Z"
    />
    <path
      fill="#0C8CE0"
      fillRule="evenodd"
      d="M16.5 6h-5.234v5.234H16.5V6Zm-5.234-2a2 2 0 0 0-2 2v5.234a2 2 0 0 0 2 2H16.5a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-5.234Z"
      clipRule="evenodd"
    />
  </svg>
);
export default SvgIconCollapse;
