import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconReadMore = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <clipPath id="icon-read-more_svg__a">
        <path className="icon-read-more_svg__a" d="M0 0h22v22H0z" />
      </clipPath>
      <style>
        {
          '\n      .icon-read-more_svg__a,.icon-read-more_svg__c{fill:#0c8ce0}.icon-read-more_svg__a{opacity:0}.icon-read-more_svg__b{clip-path:url(#icon-read-more_svg__a)}\n    '
        }
      </style>
    </defs>
    <g transform="translate(-312 -227)" className="icon-read-more_svg__b">
      <rect
        className="icon-read-more_svg__c"
        width={14}
        height={2}
        rx={1}
        transform="translate(316 233)"
      />
      <rect
        className="icon-read-more_svg__c"
        width={10}
        height={2}
        rx={1}
        transform="translate(316 237)"
      />
      <rect
        className="icon-read-more_svg__c"
        width={10}
        height={2}
        rx={1}
        transform="translate(316 241)"
      />
    </g>
  </svg>
);

export default SvgIconReadMore;
