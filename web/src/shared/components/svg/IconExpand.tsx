import * as React from 'react';
import type { SVGProps } from 'react';
const SvgIconExpand = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} fill="none" {...props}>
    <path
      fill="#0C8CE0"
      d="M18 9V5a1 1 0 0 0-1-1h-4a1 1 0 1 0 0 2h1.587L10.93 9.657a1 1 0 0 0 1.414 1.414L16 7.415V9a1 1 0 1 0 2 0Z"
    />
    <path
      fill="#0C8CE0"
      fillRule="evenodd"
      d="M10 17v-4a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v4a1 1 0 0 0 1 1h4a1 1 0 0 0 1-1Zm-4-3v2h2v-2H6Z"
      clipRule="evenodd"
    />
  </svg>
);
export default SvgIconExpand;
