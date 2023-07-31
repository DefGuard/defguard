import type { BaseTranslation } from '../i18n-types';

const en: BaseTranslation = {
  messages: {
    error: 'Error has occurred.',
    success: 'Operation succeeded',
    successClipboard: 'Copied to clipboard',
    errorVersion: 'Failed to get application version.',
    errorLicense: 'Failed to get license.',
    clipboardError: 'Clipboard is not accessible.',
  },
  modals: {
    changePasswordSelf: {
      title: 'Change password',
      messages: {
        success: 'Password changed',
        error: 'Failed to changed password',
      },
      form: {
        labels: {
          newPassword: 'New password',
          oldPassword: 'Current password',
          repeat: 'Confirm new password',
        },
      },
      controls: {
        submit: 'Change password',
        cancel: 'Cancel',
      },
    },
    deleteNetwork: {
      title: 'Delete {name:string} location',
      subTitle: 'This action will permanently delete this location.',
      submit: 'Delete location',
      cancel: 'Cancel',
    },
    changeWebhook: {
      messages: {
        success: 'Webhook changed.',
      },
    },
    manageWebAuthNKeys: {
      title: 'Security keys',
      messages: {
        deleted: 'WebAuthN key deleted.',
        duplicateKeyError: 'Key is already registered',
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
      message: 'Do you want to delete {deviceName} device ?',
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
        viewTitle: 'Device configuration',
        steps: {
          config: {
            messages: {
              copyConfig: 'Config copied to clipboard',
            },
            helpers: {
              warningAutoMode: `
        <p>
          Please be advised that you have to download the configuration now,
          since <strong>we do not</strong> store your private key. After this
          dialog is closed, you <strong>will not be able</strong> to get your
          full configuration file (with private keys, only blank template).
        </p>
`,
              warningManualMode: `
        <p>
          Please be advised that configuration provided here <strong> does not include private key and uses public key to fill it's place </strong> you will need to repalce it on your own for configuration to work properly.
        </p>
`,
              qrHelper: `
          <p>
            You can setup your device faster with wireguard application by scanning this QR code.
          </p>`,
            },
            qrInfo:
              'Use provided configuration file below by scanning QR Code or importing it as file on your devices WireGuard instance.',
            inputNameLabel: 'Device Name',
            qrLabel: 'WireGuard Config File',
            qrCardTitle: 'WireGuard Config for location:',
          },
          setup: {
            infoMessage: `
        <p>
          You need to configure WireguardVPN on your device, please visit&nbsp;
          <a href="{addDevicesDocs:string}">documentation</a> if you don&apos;t know how to do it.
        </p>
`,
            options: {
              auto: 'Generate key pair',
              manual: 'Use my own public key',
            },
            form: {
              submit: 'Generate configuration',
              fields: {
                name: {
                  label: 'Device Name',
                },
                publicKey: {
                  label: 'Provide Your Public Key',
                },
              },
              errors: {
                name: {
                  duplicatedName: 'Device with this name already exists',
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
    deleteProvisioner: {
      title: 'Delete provisioner',
      controls: {
        submit: 'Delete provisioner',
      },
      message: 'Do you want to delete {id: string} provisioner?',
      messages: {
        success: '{provisioner: string} deleted.',
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
                <b>gpg --card-edit </b> before provisioning.`,
      selectionLabel: 'Select one of the following provisioners to provision a YubiKey:',
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
    webhookModal: {
      title: {
        addWebhook: 'Add webhook.',
        editWebhook: 'Edit webhook',
      },
      messages: {
        clientIdCopy: 'Client ID copied.',
        clientSecretCopy: 'Client secret copied.',
      },
      form: {
        triggers: 'Trigger events:',
        messages: {
          successAdd: 'Webhook created.',
          successModify: 'Webhook modified.',
        },
        error: {
          urlRequired: 'URL is required.',
          validUrl: 'Must be a valid URL.',
          scopeValidation: 'Must have at least one trigger.',
          tokenRequired: 'Token is required.',
        },
        fields: {
          description: {
            label: 'Description',
            placeholder: 'Webhook to create gmail account on new user',
          },
          token: {
            label: 'Secret token',
            placeholder: 'Authorization token',
          },
          url: {
            label: 'Webhook URL',
            placeholder: 'https://example.com/webhook',
          },
          userCreated: {
            label: 'New user Created',
          },
          userDeleted: {
            label: 'User deleted',
          },
          userModified: {
            label: 'User modified',
          },
          hwkeyProvision: {
            label: 'User Yubikey provision',
          },
        },
      },
    },
    deleteWebhook: {
      title: 'Delete webhook',
      message: 'Do you want to delete {name: string} webhook ?',
      submit: 'Delete',
      messages: {
        success: 'Webhook deleted.',
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
      failedToFetchUserData: 'Could not get user information.',
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
          publicIP: 'Public IP',
          connectedThrough: 'Connected through',
          connectionDate: 'Connected date',
          lastLocation: 'Last connected from',
          lastConnected: 'Last connected',
          assignedIp: 'Assigned IP',
          active: 'active',
          noData: 'Never connected',
        },
        edit: {
          edit: 'Edit device',
          delete: 'Delete device',
          showConfigurations: 'Show configuration',
        },
      },
    },
    wallets: {
      messages: {
        addressCopied: 'Address copied.',
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
          copyAddress: 'Copy address',
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
      overview: 'VPN Overview',
      users: 'Users',
      provisioners: 'YubiKeys',
      webhooks: 'Webhooks',
      openId: 'OpenID Apps',
      myProfile: 'My Profile',
      settings: 'Settings',
      logOut: 'Log out',
    },
    mobileTitles: {
      wizard: 'Create location',
      users: 'Users',
      settings: 'Defguard Global Settings',
      user: 'User Profile',
      provisioners: 'Yubikey',
      webhooks: 'Webhooks',
      openId: 'OpenId Apps',
      overview: 'Location Overview',
      networkSettings: 'Edit Location',
    },
    copyright: 'Copyright \u00A9 2023 ',
    version: {
      open: 'Application version: {version: string}',
      closed: 'v {version: string}',
    },
  },
  form: {
    download: 'Download',
    copy: 'Copy',
    saveChanges: 'Save changes',
    submit: 'Submit',
    login: 'Sign in',
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
      portMax: 'Maximum port is 65535.',
      endpoint: 'Enter a valid endpoint.',
      address: 'Enter a valid address.',
      validPort: 'Enter a valid port.',
      validCode: 'Code should have 6 digits.',
      allowedIps: 'Only valid IP or domain is allowed.',
      startFromNumber: 'Cannot start from number',
      repeat: `Fields don't match`,
    },
    floatingErrors: {
      title: 'Please correct the following:',
    },
  },
  components: {
    gatewaysStatus: {
      label: 'Gateways',
      states: {
        connected: 'All connected',
        partial: 'One or more are not working',
        disconnected: 'Disconnected',
        error: 'Retrieving connections failed',
        loading: 'Retrieving connections',
      },
      messages: {
        error: 'Failed to get gateways status',
        deleteError: 'Failed to delete gateway',
      },
    },
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
      challengeSuccess: 'Challenge message changed',
    },
    modulesVisibility: {
      header: 'Modules Visibility',
      helper: `<p>
            If your not using some modules you can disable their visibility.
          </p>
          <a href="{documentationLink:string}" target="_blank">
            Read more in documentation.
          </a>`,
      fields: {
        wireguard_enabled: {
          label: 'Wireguard VPN',
        },
        webhooks_enabled: {
          label: 'Webhooks',
        },
        worker_enabled: {
          label: 'Yubikey provisioning',
        },
        openid_enabled: {
          label: 'OpenID Connect',
        },
      },
    },

    defaultNetworkSelect: {
      header: 'Default location view',
      helper: `<p>Here you can change your default location view.</p>
          <a href="{documentationLink:string}" target="_blank">
            Read more in documentation.
          </a>`,
      filterLabels: {
        grid: 'Grid view',
        list: 'List view',
      },
    },
    web3Settings: {
      header: 'Web3 / Wallet connect',
      fields: {
        signMessage: {
          label: 'Default sign message template',
        },
      },
      controls: {
        save: 'Save changes',
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
            helper: '<p>Maximum picture size is 250x100  px</p>',
            placeholder: 'Default image',
          },
          navLogoUrl: {
            label: 'Menu & navigation small logo',
            helper: '<p>Maximum picture size is 100x100 px</p>',
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
          <a href="{documentationLink:string}" target="_blank">
            Read more in documentation.
          </a>
			`,
    },
    smtp: {
      header: 'SMTP Settings',
      form: {
        title: 'Settings',
        fields: {
          server: {
            label: 'Server address',
            placeholder: 'Address',
          },
          port: {
            label: 'Server port',
            placeholder: 'Port',
          },
          tls: {
            label: 'Use TLS',
          },
          user: {
            label: 'Server username',
            placeholder: 'Username',
          },
          password: {
            label: 'Server password',
            placeholder: 'Password',
          },
          sender: {
            label: 'Sender email address',
            placeholder: 'Address',
            helper: `
              <p>
                System messages will be sent from this address.
                E.g. no-reply@my-company.com.
              </p>
            `,
          },
        },
        controls: {
          submit: 'Save changes',
        },
      },
      test_form: {
        title: 'Send test email',
        fields: {
          to: {
            label: 'Address',
            placeholder: 'Address',
          },
        },
        controls: {
          submit: 'Send',
          success: 'Test email sent successfully',
        },
      },
      helper: `
        <p>
          Here you can configure SMTP server used to send system messages to the users.
        </p>
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
      body: `
Before contacting or submitting any issues to GitHub please get familiar with Defguard documentation available at [defguard.gitbook.io/defguard](https://defguard.gitbook.io/defguard/)

To submit:
* Bugs - please go to [GitHub](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=bug&template=bug_report.md&title=)
* Feature request - please go to [GitHub](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=feature&template=feature_request.md&title=)

Any other requests you can reach us at: support@defguard.net
`,
    },
  },
  openidOverview: {
    pageTitle: 'OpenID Apps',
    search: {
      placeholder: 'Find apps',
    },
    filterLabels: {
      all: 'All apps',
      enabled: 'Enabled',
      disabled: 'Disabled',
    },
    clientCount: 'All apps',
    addNewApp: 'Add new',
    list: {
      headers: {
        name: 'Name',
        status: 'Status',
        actions: 'Actions',
      },
      editButton: {
        edit: 'Edit app',
        delete: 'Delete app',
        disable: 'Disable',
        enable: 'Enable',
        copy: 'Copy client ID',
      },
      status: {
        enabled: 'Enabled',
        disabled: 'Disabled',
      },
    },
    messages: {
      copySuccess: 'Client ID copied.',
      noLicenseMessage: "You don't have a license for this feature.",
      noClientsFound: 'No results found.',
    },
    deleteApp: {
      title: 'Delete app',
      message: 'Do you want to delete {appName: string} app ?',
      submit: 'Delete app',
      messages: {
        success: 'App deleted.',
      },
    },
    enableApp: {
      messages: {
        success: 'App enabled.',
      },
    },
    disableApp: {
      messages: {
        success: 'App disabled.',
      },
    },
    modals: {
      openidClientModal: {
        title: {
          addApp: 'Add Application',
          editApp: 'Edit {appName: string} app',
        },
        scopes: 'Scopes:',
        messages: {
          clientIdCopy: 'Client ID copied.',
          clientSecretCopy: 'Client secret copied.',
        },
        form: {
          messages: {
            successAdd: 'App created.',
            successModify: 'App modified.',
          },
          error: {
            urlRequired: 'URL is required.',
            validUrl: 'Must be a valid URL.',
            scopeValidation: 'Must have at least one scope.',
          },
          fields: {
            name: {
              label: 'App name',
            },
            redirectUri: {
              label: 'Redirect URL {count: number}',
              placeholder: 'https://example.com/redirect',
            },
            openid: {
              label: 'OpenID',
            },
            profile: {
              label: 'Profile',
            },
            email: {
              label: 'Email',
            },
            phone: {
              label: 'Phone',
            },
          },
          controls: {
            addUrl: 'Add URL',
          },
        },
        clientId: 'Client ID',
        clientSecret: 'Client secret',
      },
    },
  },
  webhooksOverview: {
    pageTitle: 'Webhooks',
    search: {
      placeholder: 'Find webhooks by url',
    },
    filterLabels: {
      all: 'All webhooks',
      enabled: 'Enabled',
      disabled: 'Disabled',
    },
    webhooksCount: 'All webhooks',
    addNewWebhook: 'Add new',
    noWebhooksFound: 'No webhooks found.',
    list: {
      headers: {
        name: 'Name',
        description: 'Description',
        status: 'Status',
        actions: 'Actions',
      },
      editButton: {
        edit: 'Edit',
        delete: 'Delete webhook',
        disable: 'Disable',
        enable: 'Enable',
      },
      status: {
        enabled: 'Enabled',
        disabled: 'Disabled',
      },
    },
  },
  provisionersOverview: {
    pageTitle: 'Provisioners',
    search: {
      placeholder: 'Find provisioners',
    },
    filterLabels: {
      all: 'All',
      available: 'Available',
      unavailable: 'Unavailable',
    },
    provisionersCount: 'All provisioners',
    noProvisionersFound: 'No provisioners found.',
    noLicenseMessage: "You don't have a license for this feature.",
    provisioningStation: {
      header: 'YubiKey provisioning station',
      cardTitle: 'Provisioning station setup command',
      content: `In order to be able to provision your YubiKeys, first you need to set up
        physical machine with USB slot. Run provided command on your chosen
        machine to register it and start provisioning your keys.`,
    },
    noLicenseBox: `<p>
              <strong>YubiKey module</strong>
            </p>
            <br />
            <p>This is enterprise module for YubiKey</p>
            <p>management and provisioning.</p>`,
    list: {
      headers: {
        name: 'Name',
        ip: 'IP address',
        status: 'Status',
        actions: 'Actions',
      },
      editButton: {
        delete: 'Delete provisioner',
      },
      status: {
        available: 'Available',
        unavailable: 'Unavailable',
      },
    },
    messages: {
      codeCopied: 'Command copied.',
    },
  },
  openidAllow: {
    header: '{name: string} would like to:',
    scopes: {
      openid: 'Use your profile data for future logins.',
      profile: 'Know basic information from your profile like name, profile picture etc.',
      email: 'Know your email address.',
      phone: 'Know your phone number.',
    },
    controls: {
      accept: 'Accept',
      cancel: 'Cancel',
    },
  },
  networkOverview: {
    pageTitle: 'Location overview',
    controls: {
      editNetworks: 'Edit Locations settings',
      selectNetwork: {
        placeholder: 'Loading locations',
      },
    },
    filterLabels: {
      grid: 'Grid view',
      list: 'List view',
    },
    stats: {
      currentlyActiveUsers: 'Currently active users',
      currentlyActiveDevices: 'Currently active devices',
      activeUsersFilter: 'Active users in {hour: number}H',
      activeDevicesFilter: 'Active devices in {hour: number}H',
      totalTransfer: 'Total transfer:',
      activityIn: 'Activity in {hour: number}H',
      in: 'In:',
      out: 'Out:',
      gatewayDisconnected: 'Gateway disconnected',
    },
  },
  connectedUsersOverview: {
    pageTitle: 'Connected users',
    noUsersMessage: 'Currently there are no connected users',
    userList: {
      username: 'Username',
      device: 'Device',
      connected: 'Connected',
      deviceLocation: 'Device location',
      networkUsage: 'Network usage',
    },
  },
  networkPage: {
    pageTitle: 'Edit Location',
    addNetwork: '+ Add new location',
    controls: {
      networkSelect: {
        label: 'Location choice',
      },
    },
  },
  activityOverview: {
    header: 'Activity stream',
    noData: 'Currently there is no activity detected',
  },
  networkConfiguration: {
    messages: {
      delete: {
        success: 'Network delted',
        error: 'Failed to delete network',
      },
    },
    header: 'Location configuration',
    importHeader: 'Location import',
    form: {
      helpers: {
        address:
          'Based on this address VPN network address will be defined, eg. 10.10.10.1/24 (and VPN network will be: 10.10.10.0/24)',
        gateway: 'Gateway public address, used by VPN users to connect',
        dns: 'Specify the DNS resolvers to query when the wireguard interface is up.',
        allowedIps:
          'List of addresses/masks that should be routed through the VPN network.',
        allowedGroups:
          'By default, all users will be allowed to connect to this location. If you want to restrict access to this location to a specific group, please select it below.',
      },
      messages: {
        networkModified: 'Location modified.',
        networkCreated: 'Location created',
      },
      fields: {
        name: {
          label: 'Location name',
        },
        address: {
          label: 'Gateway VPN IP address and netmask',
        },
        endpoint: {
          label: 'Gateway address',
        },
        allowedIps: {
          label: 'Allowed Ips',
        },
        port: {
          label: 'Gateway port',
        },
        dns: {
          label: 'DNS',
        },
        allowedGroups: {
          label: 'Allowed groups',
          placeholder: 'All groups',
        },
      },
      controls: {
        submit: 'Save changes',
        cancel: 'Back to Overview',
        delete: 'Remove location',
      },
    },
  },
  gatewaySetup: {
    header: 'Gateway server setup',
    card: {
      title: 'Docker based gateway setup',
    },
    controls: {
      status: 'Check connection status',
    },
    messages: {
      runCommand: `
          <p>
            Defguard requires to deploy a gateway node to control wireguard VPN on the vpn server.
            More details can be found in the <a href="{setupGatewayDocs:string}" target="_blank">documentation</a>.
            There are several ways to deploy the gateway server,
            below is a Docker based example, for other examples please visit <a href="{setupGatewayDocs:string}" target="_blank">documentation</a>.
          </p>`,
      createNetwork: `
          <p>
            Please create the network before running the gateway process.
          </p>`,
      noConnection: `<p>No connection established, please run provided command.</p>`,
      connected: `<p>Gateway connected.</p>`,
      statusError: 'Failed to get gateway status',
    },
  },
  loginPage: {
    pageTitle: 'Enter your credentials',
    mfa: {
      title: 'Two-factor authentication',
      controls: {
        useAuthenticator: 'Use Authenticator app instead',
        useWallet: 'Use your wallet instead',
        useWebauthn: 'Use security key instead',
        useRecoveryCode: 'Use recovery code instead',
      },
      totp: {
        header: 'Use code from your authentication app and click button to proceed.',
        form: {
          fields: {
            code: {
              placeholder: 'Enter Authenticator code',
            },
          },
          controls: {
            submit: 'Use authenticator code',
          },
        },
      },
      recoveryCode: {
        header: 'Enter one of active recovery codes and click button to log in.',
        form: {
          fields: {
            code: {
              placeholder: 'Recovery code',
            },
          },
          controls: {
            submit: 'Use recovery code',
          },
        },
      },
      wallet: {
        header:
          'Use your crypto wallet to sign in, please sign message in your wallet app or extension.',
        controls: {
          submit: 'Use your wallet',
        },
        messages: {
          walletError: 'Wallet was disconnected during signing process.',
          walletErrorMfa:
            'Wallet is not authorized for MFA login. Please use authorized wallet.',
        },
      },
      webauthn: {
        header: 'When you are ready to authenticate, press the button below.',
        controls: {
          submit: 'Use security key',
        },
        messages: {
          error: 'Failed to read key. Please try again.',
        },
      },
    },
  },
  wizard: {
    completed: 'Location setup completed',
    configuration: {
      successMessage: 'Location created',
    },
    welcome: {
      header: 'Welcome to location wizard!',
      sub: 'Before you start using VPN you need to setup your first location. When in doubt click on <React> icon.',
      button: 'Setup location',
    },
    navigation: {
      top: 'Location setup',
      titles: {
        welcome: 'Location setup',
        choseNetworkSetup: 'Chose Location setup',
        importConfig: 'Import existing location',
        manualConfig: 'Configure location',
        mapDevices: 'Map imported devices',
      },
      buttons: {
        next: 'Next',
        back: 'Back',
      },
    },
    deviceMap: {
      messages: {
        crateSuccess: 'Devices added',
        errorsInForm: 'Please fill marked fields.',
      },
      list: {
        headers: {
          deviceName: 'Device Name',
          deviceIP: 'IP',
          user: 'User',
        },
      },
    },
    wizardType: {
      manual: {
        title: 'Manual Configuration',
        description: 'Manual location configuration',
      },
      import: {
        title: 'Import From File',
        description: 'Import from WireGuard config file',
      },
      createNetwork: 'Create location',
    },
    common: {
      select: 'Select',
    },
    locations: {
      form: {
        name: 'Name',
        ip: 'IP address',
        user: 'User',
        fileName: 'File',
        selectFile: 'Select file',
        messages: { devicesCreated: 'Devices created' },
        validation: { invalidAddress: 'Invalid address' },
      },
    },
  },
  layout: {
    select: {
      addNewOptionDefault: 'Add new +',
    },
  },
  redirectPage: {
    title: 'You have been logged in',
    subtitle: 'You will be redirected in a moment...',
  },
};

export default en;
