import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconEditHover = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-edit-hover_svg__a">
        <path
          className="icon-edit-hover_svg__a"
          transform="translate(627 854)"
          d="M0 0h22v22H0z"
        />
      </clipPath>
      <style>
        {
          '\n      .icon-edit-hover_svg__a{fill:#899ca8;opacity:0}.icon-edit-hover_svg__b{clip-path:url(#icon-edit-hover_svg__a)}.icon-edit-hover_svg__c{fill:#0c8ce0}\n    '
        }
      </style>
    </defs>
    <g className="icon-edit-hover_svg__b">
      <path
        className="icon-edit-hover_svg__c"
        d="M15.781 20H6.219A4.224 4.224 0 0 1 2 15.781V6.219A4.224 4.224 0 0 1 6.219 2h9.563A4.224 4.224 0 0 1 20 6.219v9.563A4.224 4.224 0 0 1 15.781 20ZM6.219 3.406a2.816 2.816 0 0 0-2.813 2.813v9.563a2.816 2.816 0 0 0 2.813 2.812h9.563a2.816 2.816 0 0 0 2.812-2.812V6.219a2.816 2.816 0 0 0-2.812-2.812Zm.6 10.969a.7.7 0 0 1-.69-.841l.4-1.992a5.069 5.069 0 0 1 1.391-2.6l3.375-3.378a2.566 2.566 0 1 1 3.629 3.629l-3.379 3.38a5.069 5.069 0 0 1-2.6 1.391l-1.992.4a.7.7 0 0 1-.138.014Zm6.293-8.156a1.153 1.153 0 0 0-.82.34L8.91 9.937a3.666 3.666 0 0 0-1.01 1.881l-.191.958.958-.191a3.666 3.666 0 0 0 1.88-1.006L13.93 8.2a1.166 1.166 0 0 0 0-1.641 1.153 1.153 0 0 0-.82-.34Zm3.938 10.266a.7.7 0 0 0-.7-.7H5.656a.703.703 0 0 0 0 1.406h10.688a.7.7 0 0 0 .703-.707Z"
      />
    </g>
  </svg>
);

export default SvgIconEditHover;
