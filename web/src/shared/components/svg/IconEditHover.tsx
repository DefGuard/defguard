import type { SVGProps } from 'react';
const SvgIconEditHover = (props: SVGProps<SVGSVGElement>) => (
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
            fill: '#899ca8',
            opacity: 0,
          }}
          transform="translate(627 854)"
        />
      </clipPath>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
    >
      <path
        d="M13.781 18H4.219A4.224 4.224 0 0 1 0 13.781V4.219A4.224 4.224 0 0 1 4.219 0h9.563A4.224 4.224 0 0 1 18 4.219v9.563A4.224 4.224 0 0 1 13.781 18ZM4.219 1.406a2.816 2.816 0 0 0-2.813 2.813v9.563a2.816 2.816 0 0 0 2.813 2.812h9.563a2.816 2.816 0 0 0 2.812-2.812V4.219a2.816 2.816 0 0 0-2.812-2.812Zm.6 10.969a.7.7 0 0 1-.69-.841l.4-1.992a5.069 5.069 0 0 1 1.391-2.6l3.375-3.378a2.566 2.566 0 1 1 3.629 3.629l-3.379 3.38a5.069 5.069 0 0 1-2.6 1.391l-1.992.4a.7.7 0 0 1-.138.014Zm6.293-8.156a1.153 1.153 0 0 0-.82.34L6.91 7.937A3.666 3.666 0 0 0 5.9 9.818l-.191.958.958-.191a3.666 3.666 0 0 0 1.88-1.006L11.93 6.2a1.166 1.166 0 0 0 0-1.641 1.153 1.153 0 0 0-.82-.34Zm3.938 10.266a.7.7 0 0 0-.7-.7H3.656a.7.7 0 0 0 0 1.406h10.688a.7.7 0 0 0 .703-.707Z"
        style={{
          fill: '#0c8ce0',
        }}
        transform="translate(2 2)"
      />
    </g>
  </svg>
);
export default SvgIconEditHover;
