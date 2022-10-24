import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconSearchHover = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-search-hover_svg__a">
        <path className="icon-search-hover_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-search-hover_svg__a,.icon-search-hover_svg__c{fill:#0c8ce0}.icon-search-hover_svg__a{opacity:0}.icon-search-hover_svg__b{clip-path:url(#icon-search-hover_svg__a)}\n    '
        }
      </style>
    </defs>
    <g className="icon-search-hover_svg__b">
      <path
        className="icon-search-hover_svg__c"
        d="M10.379 4a6.375 6.375 0 0 1 4.951 10.4L18 17.067l-.933.933-2.667-2.67A6.378 6.378 0 1 1 10.379 4Zm0 11.438a5.059 5.059 0 1 0-5.059-5.059 5.065 5.065 0 0 0 5.059 5.059Z"
      />
    </g>
  </svg>
);

export default SvgIconSearchHover;
