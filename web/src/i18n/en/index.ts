import type { BaseTranslation } from '../i18n-types';

const en: BaseTranslation = {
  messages: {
    error: 'Error has occurred.',
    success: 'Operation succeeded',
    errorVersion: 'Failed to get application version.',
    errorLicense: 'Failed to get license.',
    clipboardError: 'Clipboard is not accessible.',
  },
  modals: {
    manageWebAuthNKeys: {
      title: 'Security keys',
      messages: {
        deleted: 'WebAuthN key deleted.',
      },
      infoMessage: `
        <p>
          Security keys can be used as your second factor of authentication
          instead of a verification code. Learn more about configuring a
          security key.
        </p>
`,
      form: {
        messages: {
          success: 'Security key added.',
        },
        fields: {
          name: {
            label: 'New key name',
          },
        },
        controls: {
          submit: 'Add new Key',
        },
      },
    },
    recoveryCodes: {
      title: 'Recovery codes',
      submit: 'I have saved my codes',
      messages: {
        copied: 'Codes copied.',
      },
      infoMessage: `
        <p>
          Treat your recovery codes with the same level of attention as you
          would your password! We recommend saving them with a password manager
          such as Lastpass, bitwarden or Keeper.
        </p>
`,
    },
    registerTOTP: {
      title: 'Authenticator App Setup',
      infoMessage: `
        <p>
          To setup your MFA, scan this QR code with your authenticator app, then
          enter the code in the field below:
        </p>
`,
      messages: {
        totpCopied: 'TOTP path copied.',
        success: 'TOTP Enabled',
      },
      copyPath: 'Copy TOTP path',
      form: {
        fields: {
          code: {
            label: 'Authenticator code',
            error: 'Code is invalid',
          },
        },
        controls: {
          submit: 'Verify code',
        },
      },
    },
    editDevice: {
      title: 'Edit device',
      messages: {
        success: 'Device updated.',
      },
      form: {
        fields: {
          name: {
            label: 'Device Name',
          },
          publicKey: {
            label: 'Device Public Key (Wireguard)',
          },
        },
        controls: {
          submit: 'Edit device',
        },
      },
    },
    deleteDevice: {
      title: 'Delete device',
      message: 'Do you want to delete {deviceName: string} device ?',
      submit: 'Delete device',
      messages: {
        success: 'Device deleted.',
      },
    },
    addDevice: {
      messages: {
        success: 'Device added.',
      },
      web: {
        title: 'Add device',
        steps: {
          config: {
            messages: {
              copyConfig: 'Config copied to clipboard',
            },
            inputNameLabel: 'Device Name',
            warningMessage: `
        <p>
          Please be advised that you have to download the configuration now,
          since <strong>we do not</strong> store your private key. After this
          dialog is closed, you <strong>will not be able</strong> to get your
          full configuration file (with private keys, only blank template).
        </p>
`,
            qrInfo:
              'Use provided configuration file below by scanning QR Code or importing it as file on your devices WireGuard instance.',
            qrLabel: 'WireGuard Config File',
            qrHelper: `
          <p>
            This configuration file can be scanned, copied or downloaded, but
            needs to be used
            <strong>on your device that you are adding now.</strong>
            <a>Read more in documentation.</a>
          </p>`,
            qrCardTitle: 'WireGuard Config',
          },
          setup: {
            infoMessage: `
        <p>
          You need to configure WireguardVPN on your device, please visit&nbsp;
          <a href="">documentation</a> if you don&apos;t know how to do it.
        </p>
`,
            options: {
              auto: 'Generate key pair',
              manual: 'Use my own public key',
            },
            form: {
              submit: 'Generate Config',
              fields: {
                name: {
                  label: 'Device Name',
                },
                publicKey: {
                  label: 'Provide Your Public Key',
                },
              },
            },
          },
        },
      },
      desktop: {
        title: 'Add current device',
        form: {
          submit: 'Add this device',
          fields: {
            name: {
              label: 'Name',
            },
          },
        },
      },
    },
    addWallet: {
      title: 'Add wallet',
      infoBox: 'In order to add a ETH wallet you will need to sign message.',
      form: {
        fields: {
          name: {
            placeholder: 'Wallet name',
            label: 'Name',
          },
          address: {
            placeholder: 'Wallet address',
            label: 'Address',
          },
        },
        controls: {
          submit: 'Add wallet',
        },
      },
    },
    keyDetails: {
      title: 'YubiKey details',
      downloadAll: 'Download all keys',
    },
    deleteUser: {
      title: 'Delete account',
      controls: {
        submit: 'Delete account',
      },
      message: 'Do you want to delete {username: string} account permanently ?',
      messages: {
        success: '{username: string} deleted.',
      },
    },
    changeUserPassword: {
      messages: {
        success: 'Password changed.',
      },
      title: 'Change user password',
      form: {
        controls: {
          submit: 'Save new password',
        },
        fields: {
          newPassword: {
            label: 'New password',
          },
          confirmPassword: {
            label: 'Repeat password',
          },
        },
      },
    },
    provisionKeys: {
      title: 'Yubikey provisioning:',
      infoBox: `The selected provisioner must have a <b>clean</b> YubiKey
                plugged in be provisioned. To clean a used YubiKey
                <b>gpg-card factory reset</b> before provisioning.`,
      selectionLabel:
        'Select one of the following provisioners to provision a YubiKey:',
      noData: {
        workers: 'No workers found, waiting...',
      },
      controls: {
        submit: 'Provision YubiKey',
      },
      messages: {
        success: 'Keys provisioned',
        errorStatus: 'Error while getting worker status.',
      },
    },
    addUser: {
      title: 'Add new user',
      form: {
        submit: 'Add user',
        fields: {
          username: {
            placeholder: 'login',
            label: 'Login',
          },
          password: {
            placeholder: 'Password',
            label: 'Password',
          },
          email: {
            placeholder: 'User e-mail',
            label: 'User e-mail',
          },
          firstName: {
            placeholder: 'First name',
            label: 'First name',
          },
          lastName: {
            placeholder: 'Last name',
            label: 'Last name',
          },
          phone: {
            placeholder: 'Phone',
            label: 'Phone',
          },
        },
      },
    },
  },
  userPage: {
    title: {
      view: 'User Profile',
      edit: 'Edit User Profile',
    },
    messages: {
      editSuccess: 'User updated.',
    },
    userDetails: {
      header: 'Profile Details',
      messages: {
        deleteApp: 'App and all tokens deleted.',
      },
      fields: {
        username: {
          label: 'Username',
        },
        firstName: {
          label: 'First name',
        },
        lastName: {
          label: 'Last name',
        },
        phone: {
          label: 'Phone number',
        },
        email: {
          label: 'E-mail',
        },
        groups: {
          label: 'User groups',
          noData: 'No groups',
        },
        apps: {
          label: 'Authorized apps',
          noData: 'No authorized apps',
        },
      },
    },
    userAuthInfo: {
      header: 'Password and authentication',
      password: {
        header: 'Password settings',
        changePassword: 'Change password',
      },
      recovery: {
        header: 'Recovery options',
        codes: {
          label: 'Recovery Codes',
          viewed: 'Viewed',
        },
      },
      mfa: {
        header: 'Two-factor methods',
        edit: {
          disable: 'Disable MFA',
        },
        messages: {
          mfaDisabled: 'MFA disabled.',
          OTPDisabled: 'One time password disabled.',
          changeMFAMethod: 'MFA method changed',
        },
        securityKey: {
          singular: 'security key',
          plural: 'security keys',
        },
        default: 'default',
        enabled: 'Enabled',
        disabled: 'Disabled',
        wallet: {
          singular: 'Wallet',
          plural: 'Wallets',
        },
        labels: {
          totp: 'Time based one time passwords',
          webauth: 'Security keys',
          wallets: 'Wallets',
        },
        editMode: {
          enable: 'Enable',
          disable: 'Disable',
          makeDefault: 'Make default',
          webauth: {
            manage: 'Manage security keys',
          },
        },
      },
    },
    controls: {
      editButton: 'Edit profile',
      deleteAccount: 'Delete account',
    },
    devices: {
      header: 'User devices',
      addDevice: {
        web: 'Add new device',
        desktop: 'Add this device',
      },
      card: {
        labels: {
          location: 'Last location',
          lastIpAddress: 'Last IP address',
          date: 'Date added',
        },
        edit: {
          edit: 'Edit device',
          download: 'Download config',
          delete: 'Delete device',
        },
      },
    },
    wallets: {
      messages: {
        duplicate: {
          primary: 'Connected wallet is already registered',
          sub: 'Please connect unused wallet.',
        },
      },
      header: 'User wallets',
      addWallet: 'Add new wallet',
      card: {
        address: 'Address',
        mfaBadge: 'MFA',
        edit: {
          enableMFA: 'Enable MFA',
          disableMFA: 'Disable MFA',
          delete: 'Delete',
        },
        messages: {
          deleteSuccess: 'Wallet deleted',
          enableMFA: 'Wallet MFA enabled',
          disableMFA: 'Wallet MFA disabled',
        },
      },
    },
    yubiKey: {
      header: 'User YubiKey',
      provision: 'Provision a YubiKey',
      keys: {
        pgp: 'PGP key',
        ssh: 'SSH key',
      },
      noLicense: {
        moduleName: 'YubiKey module',
        line1: 'This is enterprise module for YubiKey',
        line2: 'management and provisioning.',
      },
    },
  },
  usersOverview: {
    pageTitle: 'Users',
    search: {
      placeholder: 'Find users',
    },
    filterLabels: {
      all: 'All users',
      admin: 'Admins only',
      users: 'Users only',
    },
    usersCount: 'All users',
    addNewUser: 'Add new',
    list: {
      headers: {
        name: 'User name',
        username: 'Login',
        phone: 'Phone',
        actions: 'Actions',
      },
      editButton: {
        changePassword: 'Change password',
        edit: 'Edit account',
        provision: 'Provision YubiKey',
        delete: 'Delete account',
      },
    },
  },
  navigation: {
    bar: {
      overview: 'Overview',
      users: 'Users',
      provisioners: 'Provisioners',
      webhooks: 'Webhooks',
      openId: 'OpenID Apps',
      myProfile: 'My Profile',
      settings: 'Settings',
      logOut: 'Log out',
    },
    mobileTitles: {
      users: 'Users',
      settings: 'Defguard Global Settings',
      user: 'User Profile',
      provisioners: 'Provisioners',
      webhooks: 'Webhooks',
      openId: 'OpenId Apps',
      overview: 'Network Overview',
      networkSettings: 'Network Settings',
    },
    copyright: 'Copyright \u00A9 2023',
    version: 'Application version: {version: string}',
  },
  form: {
    download: 'Download',
    copy: 'Copy',
    saveChanges: 'Save changes',
    submit: 'Submit',
    cancel: 'Cancel',
    close: 'Close',
    placeholders: {
      password: 'Password',
      username: 'Username',
    },
    error: {
      usernameTaken: 'Username is already in use',
      invalidKey: 'Key is invalid.',
      invalid: 'Field is invalid.',
      required: 'Field is required.',
      maximumLength: 'Maximum length exceeded.',
      minimumLength: 'Minimum length not reached.',
      noSpecialChars: 'No special characters are allowed.',
      oneDigit: 'One digit required.',
      oneSpecial: 'Special character required.',
      oneUppercase: 'One uppercase character required.',
      oneLowercase: 'One lowercase character required.',
    },
  },
  components: {
    noLicenseBox: {
      footer: {
        get: 'Get an enterprise license',
        contact: 'by contacting:',
      },
    },
  },
  settingsPage: {
    title: 'Global Settings',
    messages: {
      editSuccess: 'Settings updated',
    },
    modulesVisibility: {
      header: 'Modules Visibility',
			helper: `<p>
            If your not using some modules you can disable their visibility.
          </p>
          <a href="defguard.gitbook.io" target="_blank">
            Read more in documentation.
          </a>`,
      fields: {
        wireguard_enabled: {
          label: 'Wireguard VPN',
        },
        webhooks_enabled: {
          label: 'Webhooks',
        },
        web3_enabled: {
          label: 'Web3',
        },
        worker_enabled: {
          label: 'YubiBridge',
        },
        openid_enabled: {
          label: 'OpenID connect',
        },
        oauth_enabled: {
          label: 'OAuth2',
        },
      },
    },

    defaultNetworkSelect: {
      header: 'Default network view',
      helper: `<p>Here you can change your default network view.</p>
          <a href="defguard.gitbook.io" target="_blank">
            Read more in documentation.
          </a>`,
    },
    web3Settings: {
      header: 'Web3 / Wallet connect',
      fields: {
        signMessage: {
          label: 'Default sign message template',
        },
      },
    },
    instanceBranding: {
      header: 'Instance Branding',
      form: {
        title: 'Name & Logo:',
        fields: {
          instanceName: {
            label: 'Instance name',
            placeholder: 'Defguard',
          },
          mainLogoUrl: {
            label: 'Login logo url',
            placeholder: 'Default image',
          },
          navLogoUrl: {
            label: 'Navigation Logo url',
            placeholder: 'Default image',
          },
        },
        controls: {
          restoreDefault: 'Restore default',
          submit: 'Save changes',
        },
      },
      helper: `
			      <p>
            Here you can add url of your logo and name for your defguard
            instance it will be displayed instead of defguard.
          </p>
          <a href="defguard.gitbook.io" target="_blank">
            Read more in documentation.
          </a>
			`,
    },
    licenseCard: {
      header: 'License & Support Information',
      licenseCardTitles: {
        community: 'Community',
        enterprise: 'Enterprise',
        license: 'license',
      },
      body: {
        enterprise: `
				<p> Thank you for purchasing enterprise license!</p>
				<br />
				<p>This includes following modules:</p>`,
        community: `
              <p>
                You have our community license. If you wish to get Enterprise
                license for full features set and support, please visit
                <a href="https://defguard.net">https://defguard.net</a>
              </p>
              <br />
              <p>Enterprise license includes:</p>
				`,
        agreement: 'read license agreement',
        modules: `
          <ul>
            <li>YubiBridge</li>
            <li>OpenID</li>
            <li>OpenLDAP</li>
          </ul>
          <br />`,
      },
      footer: {
        company: 'licensed to: {company: string}',
        expiration: 'expiration date: {expiration: string}',
      },
    },
    supportCard: {
      title: 'Support',
      body: {
        enterprise: `
			<p>For Enterprise support</p>
      <p>
        Please contact: 
        <a href="mailto:support@defguard.net">support@defguard.net</a>
      </p>
			<br/>
      <p>You can also visit our Community support:</p>
      <a href="https://github.com/Defguard/defguard">
        https://github.com/Defguard/defguard
      </a>
			`,
        community: `<p>For Community support Please visit:</p>
      <a href="https://github.com/Defguard/defguard">
        https://github.com/Defguard/defguard
      </a>
			`,
      },
    },
  },
};

export default en;
