import * as React from 'react';
import { SVGProps } from 'react';

const SvgIconUserList = (props: SVGProps<SVGSVGElement>) => (
  <svg xmlns="http://www.w3.org/2000/svg" width={22} height={22} {...props}>
    <defs>
      <style>
        {
          '\n      .icon-user-list_svg__a,.icon-user-list_svg__c{fill:#899ca8}.icon-user-list_svg__a{opacity:0}.icon-user-list_svg__b{opacity:.5}\n    '
        }
      </style>
    </defs>
    <g transform="translate(-312 -227)" className="icon-user-list_svg__b">
      <rect
        className="icon-user-list_svg__c"
        width={14}
        height={2}
        rx={1}
        transform="translate(316 233)"
      />
      <rect
        className="icon-user-list_svg__c"
        width={10}
        height={2}
        rx={1}
        transform="translate(316 237)"
      />
      <rect
        className="icon-user-list_svg__c"
        width={10}
        height={2}
        rx={1}
        transform="translate(316 241)"
      />
    </g>
  </svg>
);

export default SvgIconUserList;
