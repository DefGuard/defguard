import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconHourglass = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-hourglass_svg__a">
        <path className="icon-hourglass_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-hourglass_svg__a,.icon-hourglass_svg__c{fill:#899ca8}.icon-hourglass_svg__a{opacity:0}.icon-hourglass_svg__b{clip-path:url(#icon-hourglass_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-hourglass_svg__b">
      <path
        className="icon-hourglass_svg__c"
        d="M17.104 17.75h-.625v-2.311A5.07 5.07 0 0 0 13.859 11a5.07 5.07 0 0 0 2.62-4.439V4.25h.625a.625.625 0 0 0 0-1.25H5.26a.625.625 0 0 0 0 1.25h.594v2.311A5.07 5.07 0 0 0 8.475 11a5.07 5.07 0 0 0-2.62 4.439v2.311H5.26a.625.625 0 0 0 0 1.25h11.844a.625.625 0 0 0 0-1.25Zm-10-11.189V4.25h8.125v2.311a3.81 3.81 0 0 1-3.8 3.814 3.861 3.861 0 0 1-4.325-3.814Zm0 8.878a3.86 3.86 0 0 1 4.328-3.814 3.81 3.81 0 0 1 3.8 3.814v2.311H7.104Zm6.188-7.533h-4.25a.625.625 0 0 1 0-1.25h4.25a.625.625 0 1 1 0 1.25Zm.319 8.44a.625.625 0 0 0 0-.884l-1.315-1.306a1.51 1.51 0 0 0-2.125 0l-1.316 1.306a.625.625 0 1 0 .881.887l1.316-1.307a.258.258 0 0 1 .363 0l1.316 1.306a.625.625 0 0 0 .884 0Z"
      />
    </g>
  </svg>
);

export default SvgIconHourglass;
