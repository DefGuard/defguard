import type { SVGProps } from 'react';
const SvgIconSearch = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#899ca8',
          }}
        />
      </clipPath>
      <style>{'.c{fill:#899ca8}'}</style>
    </defs>
    <path
      d="M6.379 0a6.375 6.375 0 0 1 4.951 10.4L14 13.067l-.933.933-2.667-2.67A6.378 6.378 0 1 1 6.379 0Zm0 11.438A5.059 5.059 0 1 0 1.32 6.379a5.065 5.065 0 0 0 5.059 5.059Z"
      className="c"
      style={{
        clipPath: 'url(#a)',
      }}
      transform="translate(4 4)"
    />
  </svg>
);
export default SvgIconSearch;
