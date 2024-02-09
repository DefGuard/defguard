import type { SVGProps } from 'react';
const SvgIconSearchHover = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-search-hover_svg__a">
        <path
          d="M0 0h22v22H0z"
          style={{
            fill: '#0c8ce0',
            opacity: 0,
          }}
        />
      </clipPath>
      <style>{'.icon-search-hover_svg__c{fill:#0c8ce0}'}</style>
    </defs>
    <path
      d="M6.379 0a6.375 6.375 0 0 1 4.951 10.4L14 13.067l-.933.933-2.667-2.67A6.378 6.378 0 1 1 6.379 0m0 11.438A5.059 5.059 0 1 0 1.32 6.379a5.065 5.065 0 0 0 5.059 5.059"
      className="icon-search-hover_svg__c"
      style={{
        clipPath: 'url(#icon-search-hover_svg__a)',
      }}
      transform="translate(4 4)"
    />
  </svg>
);
export default SvgIconSearchHover;
