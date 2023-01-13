import type { BaseTranslation } from '../i18n-types';

const pl: BaseTranslation = {
  messages: {
    error: 'Wystąpił błąd.',
    success: 'Operacja zakończyła się sukcesem',
    succcessClipboard: 'Skopiowano do schowka',
    errorVersion: 'Nie udało się uzyskać wersji aplikacji.',
    errorLicense: 'Nie udało się uzyskać licencji.',
    clipboardError: 'Schowek nie jest dostępny.',
  },
  modals: {
    changeWebhook: {
      messages: {
        success: 'Webhook zmieniony.',
      },
    },
    manageWebAuthNKeys: {
      title: 'Klucze bezpieczeństwa',
      messages: {
        deleted: 'Klucz WebAuthN usunięty.',
      },
      infoMessage: `
        <p>
          Klucze bezpieczeństwa mogą być używane jako drugi czynnik uwierzytelniający
          zamiast kodu weryfikacyjnego. Dowiedz się więcej o konfiguracji
          klucza bezpieczeństwa.
        </p>
`,
      form: {
        messages: {
          success: 'Klucz bezpieczeństwa dodany.',
        },
        fields: {
          name: {
            label: 'Nazwa nowego klucza',
          },
        },
        controls: {
          submit: 'Dodaj nowy klucz',
        },
      },
    },
    recoveryCodes: {
      title: 'Kody odzysku',
      submit: 'Zapisałem swoje kody',
      messages: {
        copied: 'Kody skopiowane.',
      },
      infoMessage: `
        <p>
          Traktuj swoje kody odzyskiwania z takim samym poziomem uwagi jak
          jak swoje hasło! Zalecamy zapisywanie ich za pomocą menedżera haseł
          takich jak Lastpass, bitwarden czy Keeper.
        </p>
`,
    },
    registerTOTP: {
      title: 'Authenticator App Setup',
      infoMessage: `
        <p>
          Aby skonfigurować MFA, zeskanuj ten kod QR za pomocą aplikacji uwierzytelniającej, a następnie
          wprowadź kod w polu poniżej:
        </p>
`,
      messages: {
        totpCopied: 'Ścieżka TOTP skopiowana.',
        success: 'TOTP Enabled',
      },
      copyPath: 'Skopiowana ścieżka TOTP',
      form: {
        fields: {
          code: {
            label: 'Kod uwierzytelniający',
            error: 'Kod jest nieprawidłowy',
          },
        },
        controls: {
          submit: 'Weryfikuj kod',
        },
      },
    },
    editDevice: {
      title: 'Edytuj urządzenie',
      messages: {
        success: 'Urządzenie zaktualizowane.',
      },
      form: {
        fields: {
          name: {
            label: 'Nazwa urządzenia',
          },
          publicKey: {
            label: 'Klucz publiczny urządzenia (Wireguard)',
          },
        },
        controls: {
          submit: 'Edytuj urządzenie',
        },
      },
    },
    deleteDevice: {
      title: 'Usuń urządzenie',
      message: 'Czy chcesz usunąć urządzenie {deviceName: string} ?',
      submit: 'Usuń urządzenie',
      messages: {
        success: 'Urządzenie usunięte.',
      },
    },

    addDevice: {
      messages: {
        success: 'Urządzenie dodane.',
      },
      web: {
        title: 'Dodaj urządzenie',
        steps: {
          config: {
            messages: {
              copyConfig: 'Konfiguracja skopiowana do schowka.',
            },
            inputNameLabel: 'Nazwa urządzenia',
            warningMessage: `
        <p>
          Informujemy, że musisz teraz pobrać konfigurację, ponieważ <strong>nie przechowujemy Twojego klucza prywatnego</strong>.
  Po zamknięciu tego okna dialogowego <strong>nie będzie można uzyskać pełnego pliku konfiguracyjnego</strong>
					(z kluczami prywatnymi, tylko z pustym szablonem).
        </p>
`,
            qrInfo: `Użyj dostarczonego pliku konfiguracyjnego poniżej skanując QR Code lub importując go jako plik na 
						instancję WireGuard w Twoich urządzeniach.`,
            qrLabel: 'Plik konfiguracyjny Wireguard',
            qrHelper: `
          <p>
          	Ten plik konfiguracyjny można zeskanować, skopiować lub pobrać,
            <strong>ale musi być użyty na urządzeniu, które teraz dodajesz.</strong>
            <a>Przeczytaj więcej w dokumentacji.</a>
          </p>`,
            qrCardTitle: 'Konfiguracja Wireguard',
          },
          setup: {
            infoMessage: `
        <p>
          Musisz skonfigurować WireguardVPN na swoim urządzeniu, odwiedź stronę
          <a href="">documentation</a> jeśli nie wiesz jak to zrobić.
        </p>
`,
            options: {
              auto: 'Wygeneruj parę kluczy',
              manual: 'Użyj mojego własnego klucza publicznego',
            },
            form: {
              submit: 'Stwórz konfigurację',
              fields: {
                name: {
                  label: 'Nazwa urządzenia',
                },
                publicKey: {
                  label: 'Podaj swój klucz publiczny',
                },
              },
            },
          },
        },
      },
      desktop: {
        title: 'Dodaj aktualne urządzenie',
        form: {
          submit: 'Dodaj to urządzenie',
          fields: {
            name: {
              label: 'Nazwa',
            },
          },
        },
      },
    },
    addWallet: {
      title: 'Dodaj portfel',
      infoBox: 'Aby dodać portfel ETH konieczne będzie podpisanie wiadomości.',
      form: {
        fields: {
          name: {
            placeholder: 'Nazwa portfela',
            label: 'Nazwa',
          },
          address: {
            placeholder: 'Adres portfela',
            label: 'Adres',
          },
        },
        controls: {
          submit: 'Dodaj portfel',
        },
      },
    },
    keyDetails: {
      title: 'Szczegóły YubiKey',
      downloadAll: 'Pobierz wszystkie klucze',
    },
    deleteUser: {
      title: 'Usuń użytkownika',
      controls: {
        submit: 'Usuń użytkownika',
      },
      message: 'Czy chcesz trwale usunąć konto {username: string} ?',
      messages: {
        success: '{username: string} usunięte.',
      },
    },
    deleteProvisioner: {
      title: 'Usuń provisionera',
      controls: {
        submit: 'Usuń provisionera',
      },
      message: 'Czy chcesz usunąć {id: string} provisionera?',
      messages: {
        success: '{provisioner: string} usunięty.',
      },
    },
    changeUserPassword: {
      messages: {
        success: 'Hasło zmienione.',
      },
      title: 'Zmiana hasła użytkownika',
      form: {
        controls: {
          submit: 'Zapisz nowe hasło',
        },
        fields: {
          newPassword: {
            label: 'Nowe hasło',
          },
          confirmPassword: {
            label: 'Powtórz hasło',
          },
        },
      },
    },
    provisionKeys: {
      title: 'Provisionowanie YubiKeya:',
      infoBox: `Wybrany provisioner musi mieć podłączony <b>pusty</b> YubiKey.
                Aby zresetować YubiKey uruchom 
                <b>gpg --card-edit</b> przed generowaniem kluczy.`,
      selectionLabel:
        'Wybierz jeden z następujących provisionerów, aby wygenrować klucze na YubiKey:',
      noData: {
        workers: 'Nie znaleziono workerów...',
      },
      controls: {
        submit: 'Wygeneruj klucze dla YubiKey',
      },
      messages: {
        success: 'Klucze zostały przetransferowane na YubiKey',
        errorStatus: 'Wystapił błąd podczas pobierania statusu.',
      },
    },
    addUser: {
      title: 'Dodaj nowego użytkownika',
      form: {
        submit: 'Dodaj użytkownika',
        fields: {
          username: {
            placeholder: 'login',
            label: 'Login',
          },
          hasło: {
            placeholder: 'Hasło',
            label: 'Hasło',
          },
          email: {
            placeholder: 'E-mail użytkownika',
            label: 'E-mail użytkownika',
          },
          firstName: {
            placeholder: 'Imię',
            label: 'Imię',
          },
          lastName: {
            placeholder: 'Nazwisko',
            label: 'Ostatnie imię',
          },
          phone: {
            placeholder: 'Telefon',
            label: 'Telefon',
          },
        },
      },
    },
    webhookModal: {
      title: {
        addWebhook: 'Dodaj webhook',
        editWebhook: 'Edytuj webhook',
      },
      messages: {
        clientIdCopy: 'Skopiowano identyfikator klienta',
        clientSecretCopy: 'Sekret klienta skopiowany.',
      },
      form: {
        triggers: 'Zdarzenia wyzwalające:',
        messages: {
          successAdd: 'Webhook utworzony.',
          successModify: 'Webhook zmodyfikowany.',
        },
        error: {
          urlRequired: 'URL jest wymagany.',
          validUrl: 'Musi być poprawnym adresem URL.',
          scopeValidation: 'Musi mieć co najmniej jeden wyzwalacz.',
          tokenRequired: 'Token jest wymagany.',
        },
        fields: {
          description: {
            label: 'Opis',
            placeholder:
              'Webhook do tworzenia konta gmail na nowym użytkowniku',
          },
          token: {
            label: 'Secret token',
            placeholder: 'Token autoryzacyjny',
          },
          url: {
            label: 'Webhook URL',
            placeholder: 'https://example.com/webhook',
          },
          userCreated: {
            label: 'Stworzenie nowego użytkownika',
          },
          userDeleted: {
            label: 'Użytkownik usunięty',
          },
          userModified: {
            label: 'Użytkownik zmodyfikowany',
          },
          hwkeyProvision: {
            label: 'Stworzenie kluczy na YubiKey dla użytkownika',
          },
        },
      },
    },
    deleteWebhook: {
      title: 'Usuń webhook',
      message: 'Czy chcesz usunąć {name: string} webhook ?',
      submit: 'Usuń',
      messages: {
        success: 'Webhook usunięty.',
      },
    },
  },
  userPage: {
    title: {
      view: 'Profil użytkownika',
      edit: 'Edycja profilu użytkownika',
    },
    messages: {
      editSuccess: 'Użytkownik zaktualizowany.',
    },
    userDetails: {
      header: 'Szczegóły profilu',
      messages: {
        deleteApp: 'Aplikacja i wszystkie tokeny usunięte.',
      },
      fields: {
        username: {
          label: 'Nazwa użytkownika',
        },
        firstName: {
          label: 'Imię',
        },
        lastName: {
          label: 'Last name',
        },
        phone: {
          label: 'Numer telefonu',
        },
        email: {
          label: 'E-mail',
        },
        groups: {
          label: 'Grupy użytkowników',
          noData: 'Brak grup',
        },
        apps: {
          label: 'Autoryzowane aplikacje',
          noData: 'Brak autoryzowanych aplikacji',
        },
      },
    },
    userAuthInfo: {
      header: 'Hasło i uwierzytelnienie',
      password: {
        header: 'Ustawienia hasła',
        changePassword: 'Zmiana hasła',
      },
      recovery: {
        header: 'Opcje odzyskiwania danych',
        codes: {
          label: 'Kody odzyskiwania',
          viewed: 'Obejrzane',
        },
      },
      mfa: {
        header: 'Metody dwuskładnikowe',
        edit: {
          disable: 'Wyłącz MFA',
        },
        messages: {
          mfaDisabled: 'MFA wyłączone',
          OTPDisabled: 'Hasło jednorazowe wyłączone.',
          changeMFAMethod: 'Metoda MFA zmieniona',
        },
        securityKey: {
          singular: 'klucz bezpieczeństwa',
          plural: 'klucze bezpieczeństwa',
        },
        default: 'domyślny',
        enabled: 'Włączony',
        disabled: 'Wyłączony',
        wallet: {
          singular: 'Portfel',
          plural: 'Portfele',
        },
        labels: {
          totp: 'Hasła jednorazowe oparte na czasie',
          webauth: 'Klucze bezpieczeństwa',
          wallets: 'Portfele',
        },
        editMode: {
          enable: 'Włącz',
          disable: 'Wyłącz',
          makeDefault: 'Uczyń domyślnym',
          webauth: {
            manage: 'Zarządzaj kluczami bezpieczeństwa',
          },
        },
      },
    },
    controls: {
      editButton: 'Edytuj profil',
      deleteAccount: 'Usuń konto',
    },
    devices: {
      header: 'Urządzenia użytkownika',
      addDevice: {
        web: 'Dodaj nowe urządzenie',
        desktop: 'Dodaj to urządzenie',
      },
      card: {
        labels: {
          location: 'Ostatnia lokalizacja',
          lastIpAddress: 'Ostatni adres IP',
          date: 'Data dodania',
        },
        edit: {
          edit: 'Edycja urządzenia',
          download: 'Pobierz konfigurację',
          delete: 'Usuń urządzenie',
        },
      },
    },
    wallets: {
      messages: {
        duplikat: {
          primary: 'Podłączony portfel jest już zarejestrowany',
          sub: 'Proszę połączyć nieużywany portfel.',
        },
      },
      header: 'Portfele użytkowników',
      addWallet: 'Dodaj nowy portfel',
      card: {
        address: 'Adres',
        mfaBadge: 'MFA',
        edit: {
          enableMFA: 'Włącz MFA',
          disableMFA: 'Wyłącz MFA',
          delete: 'Usuń',
        },
        messages: {
          deleteSuccess: 'Portfel usunięty',
          enableMFA: 'MFA w portfelu włączone',
          disableMFA: 'MFA w portfelu wyłączone',
        },
      },
    },
    yubiKey: {
      header: 'YubiKey użytkownika',
      provision: 'Sprovisionuj YubiKey',
      keys: {
        pgp: 'Klucz PGP',
        ssh: 'Klucz SSH',
      },
      noLicense: {
        moduleName: 'Moduł YubiKey',
        line1: 'To jest płatny moduł dla YubiKey',
        line2: 'zarządzanie i provisioning.',
      },
    },
  },
  usersOverview: {
    pageTitle: 'Użytkownicy',
    search: {
      placeholder: 'Znajdź użytkowników',
    },
    filterLabels: {
      all: 'Wszyscy użytkownicy',
      admin: 'Tylko administratorzy',
      users: 'Tylko użytkownicy',
    },
    usersCount: 'Wszyscy użytkownicy',
    addNewUser: 'Dodaj użytkownika',
    list: {
      headers: {
        name: 'Nazwa użytkownika',
        username: 'Login',
        phone: 'Telefon',
        actions: 'Akcje',
      },
      editButton: {
        changePassword: 'Zmień hasło',
        edit: 'Edytuj konto',
        provision: 'Stwórz klucze na YubiKey',
        delete: 'Usuń konto',
      },
    },
  },
  navigation: {
    bar: {
      overview: 'Przegląd sieci',
      users: 'Użytkownicy',
      provisioners: 'Provisioners',
      webhooks: 'Webhooki',
      openId: 'Aplikacje OpenID',
      myProfile: 'Mój profil',
      settings: 'Ustawienia',
      logOut: 'Wyloguj się',
    },
    mobileTitles: {
      users: 'Użytkownicy',
      settings: 'Defguard ustaawienia globalne',
      user: 'Profil użytkownika',
      provisioners: 'Provisioners',
      webhooks: 'Webhooki',
      openId: 'Aplikacje OpenID',
      overview: 'Przegląd sieci',
      networkSettings: 'Ustawienia sieci',
    },
    copyright: 'Copyright \u00A9 2023',
    version: 'Wersja aplikacji: {version: string}',
  },
  form: {
    download: 'Pobierz',
    copy: 'Kopiuj',
    saveChanges: 'Zapisz zmiany',
    submit: 'Zapisz',
    login: 'Zaloguj sie',
    cancel: 'Anuluj',
    close: 'Zamknij',
    placeholders: {
      password: 'Hasło',
      username: 'Nazwa użytkownika',
    },
    error: {
      usernameTaken: 'Nazwa użytkownika jest już w użyciu',
      invalidKey: 'Klucz jest nieprawidłowy.',
      invalid: 'Pole jest nieprawidłowe.',
      required: 'Pole jest wymagane.',
      maximumLength: 'Maksymalna długość przekroczona.',
      minimumLength: 'Minimalna długość nie została osiągnięta',
      noSpecialChars: 'Nie wolno używać znaków specjalnych.',
      oneDigit: 'Wymagana jedna cyfra.',
      oneSpecial: 'Wymagany jest znak specjalny.',
      oneUppercase: 'Wymagany jeden duży znak.',
      oneLowercase: 'Wymagany jeden znak małej litery.',
      portMax: 'Maksymalny numer portu to 65535.',
      endpoint: 'Wpisz prawidłowy punkt końcowy.',
      address: 'Wprowadź poprawny adres.',
      validPort: 'Wprowadź prawidłowy port.',
      validCode: 'Kod powinien mieć 6 cyfr',
    },
  },
  components: {
    noLicenseBox: {
      footer: {
        get: 'Uzyskaj licencję enterprise',
        contact: 'poprzez kontakt:',
      },
    },
  },
  settingsPage: {
    title: 'Ustawienia globalne',
    messages: {
      editSuccess: 'Ustawienia zaktualizowane.',
      challengeSuccess: 'Zmieniono wiadomość do podpisu.',
    },
    modulesVisibility: {
      header: 'Widoczność modułów',
      helper: `<p>
			Jeśli nie używasz niektórych modułów możesz zmienić ich widoczność
          </p>
          <a href="defguard.gitbook.io" target="_blank">
					Przeczytaj więcej w dokumentacji.
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
      header: 'Domyślny widok sieci',
      helper: `<p>Tutaj możesz zmienić domyślny widok sieci.</p>
          <a href="defguard.gitbook.io" target="_blank">
					Przeczytaj więcej w dokumentacji.
          </a>`,
      filterLabels: {
        grid: 'Widok siatki',
        list: 'Widok listy',
      },
    },
    web3Settings: {
      header: 'Web3 / Wallet connect',
      fields: {
        signMessage: {
          label: 'Domyślna wiadomość do podpisu',
        },
      },
      controls: {
        save: 'Zapisz zmiany',
      },
    },
    instanceBranding: {
      header: 'Brandowanie instancji',
      form: {
        title: 'Nazwa i logo',
        fields: {
          instanceName: {
            label: 'Nazwa instancji',
            placeholder: 'Defguard',
          },
          mainLogoUrl: {
            label: 'Url loga na stronie logowania',
						helper: '<p>Maksymalna wielkość zdjęcia to 250x100 px.</p>',
            placeholder: 'Domyślny obrazek',
          },
          navLogoUrl: {
            label: 'Menu i nawigacja - małe logo',
            helper: '<p>Maksymalna wielkość zdjęcia to 100x100 px.</p>',
            placeholder: 'Domyślny obrazek',
          },
        },
        controls: {
          restoreDefault: 'Przywróć domyślne',
          submit: 'Zapisz zmiany',
        },
      },
      helper: `
			      <p>
            Tutaj możesz dodać adres url swojego logo i nazwę dla swojej instancji defguard
            będzie ona wyświetlana zamiast defguard.
          </p>
          <a href="defguard.gitbook.io" target="_blank">
					Przeczytaj więcej w dokumentacji.
          </a>
			`,
    },
    licenseCard: {
      header: 'Informacje o licencji i wsparciu technicznym',
      licenseCardTitles: {
        community: 'Community',
        enterprise: 'Enterprise',
        license: 'licencja',
      },
      body: {
        enterprise: `
				<p> Dziękujemy za zakup licencji dla przedsiębiorstw!</p>
				<br />
				<p>Obejmuje ona następujące moduły:</p>`,
        community: `
              <p>
							Masz naszą licencję community. Jeśli chcesz uzyskać licencję Enterprise odwiedź:
                <a href="https://defguard.net">https://defguard.net</a>
              </p>
              <br />
              <p>Licencja enterprise zawiera:</p>
				`,
        agreement: 'Przeczytaj umowę licencyjną',
        modules: `
          <ul>
            <li>YubiBridge</li>
            <li>OpenID</li>
            <li>OpenLDAP</li>
          </ul>
          <br />`,
      },
      footer: {
        company: 'licencjonowany dla: {company: string}',
        expiration: 'data ważności: {expiration: string}',
      },
    },
    supportCard: {
      title: 'Wsparcie',
      body: {
        enterprise: `
			<p>Po wsparcie enterprise</p>
      <p>
			Proszę kontaktuj się na:
        <a href="mailto:support@defguard.net">support@defguard.net</a>
      </p>
			<br/>
      <p>Możesz również odwiedzić nasze wsparcie dla społeczności:</p>
      <a href="https://github.com/Defguard/defguard">
        https://github.com/Defguard/defguard
      </a>
			`,
        community: `<p>W celu uzyskania wsparcia community odwiedź:</p>
      <a href="https://github.com/Defguard/defguard">
        https://github.com/Defguard/defguard
      </a>
			`,
      },
    },
  },
  openidOverview: {
    pageTitle: 'Aplikacje OpenID',
    search: {
      placeholder: 'Znajdź aplikacje',
    },
    filterLabels: {
      all: 'Wszystkie aplikacje',
      enabled: 'Włączone',
      disabled: 'Wyłączone',
    },
    clientCount: 'Wszystkie aplikacje',
    addNewApp: 'Dodaj aplikację',
    list: {
      headers: {
        name: 'Nazwa',
        status: 'Status',
        actions: 'Akcję',
      },
      editButton: {
        edit: 'Edytuj aplikację',
        delete: 'Usuń aplikację',
        disable: 'Wyłącz',
        enable: 'Włącz',
      },
      status: {
        enabled: 'Włączona',
        disabled: 'Wyłączona',
      },
    },
    messages: {
      noLicenseMessage: 'Nie masz licencji na tę funkcję.',
      noClientsFound: 'Nie znaleziono żadnych wyników.',
    },
    deleteApp: {
      title: 'Usuń aplikację',
      message: 'Czy chcesz usunąć aplikację {appName: string} ?',
      submit: 'Usuń aplikację',
      messages: {
        success: 'Aplikacja usunięta.',
      },
    },
    enableApp: {
      messages: {
        success: 'Aplikacja włączona',
      },
    },
    disableApp: {
      messages: {
        success: 'Aplikacja wyłączona',
      },
    },
    modals: {
      openidClientModal: {
        title: {
          addApp: 'Dodaj aplikację',
          editApp: 'Edytuj aplikację: {appName: string}',
        },
        scopes: 'Zakresy:',
        messages: {
          clientIdCopy: 'Client ID zostało skopiowane.',
          clientSecretCopy: 'Client secret zostało skopiowane.',
        },
        form: {
          messages: {
            successAdd: 'Aplikacja utworzona.',
            successModify: 'Aplikacja zmodyfikowana.',
          },
          error: {
            urlRequired: 'URL jest wymagany.',
            validUrl: 'Musi być poprawnym adresem URL.',
            scopeValidation: 'Musi mieć co najmniej jeden zakres.',
          },
          fields: {
            name: {
              label: 'Nazwa aplikacji',
            },
            redirectUri: {
              label: 'Przekierowujący URL {count: number}',
              placeholder: 'https://example.com/redirect',
            },
            openid: {
              label: 'OpenID',
            },
            profile: {
              label: 'Profil',
            },
            email: {
              label: 'Email',
            },
            phone: {
              label: 'Telefon',
            },
          },
          controls: {
            addUrl: 'Dodaj URL',
          },
        },
        clientId: 'Client ID',
        clientSecret: 'Client secret',
      },
    },
  },
  webhooksOverview: {
    pageTitle: 'Webhooki',
    search: {
      placeholder: 'Znajdź webhooki po adresie url',
    },
    filterLabels: {
      all: 'Wszystkie webhooki',
      enabled: 'Enabled',
      disabled: 'Disabled',
    },
    webhooksCount: 'Wszystkie webhooki',
    addNewWebhook: 'Dodaj webhook',
    noWebhooksFound: 'Nie znaleziono żadnych webhooków',
    list: {
      headers: {
        name: 'Nazwa',
        description: 'Opis',
        status: 'Status',
        actions: 'Akcję',
      },
      editButton: {
        edit: 'Edytuj',
        delete: 'Usuń webhook',
        disable: 'Wyłącz',
        enable: 'Włącz',
      },
      status: {
        enabled: 'Włączony',
        disabled: 'Wyłączony',
      },
    },
  },
  provisionersOverview: {
    pageTitle: 'Provisionery',
    search: {
      placeholder: 'Wyszukaj provisionera',
    },
    filterLabels: {
      all: 'Wszystkie',
      available: 'Dostępne',
      unavailable: 'Niedostępne',
    },
    provisionersCount: 'Wszystkie provisionery',
    noProvisionersFound: 'Nie znaleziono provisionerów.',
    noLicenseMessage: 'Nie masz licencji na tę funkcję.',
    provisioningStation: {
      header: 'Stacja provisionująca YubiKey',
      cardTitle: 'Komenda uruchamiająca stację',
      content: `Aby móc sprovisionować YubiKeya, należy najpierw skonfigurować
        fizyczną maszynę z gniazdem USB. Uruchom podane polecenie na wybranej maszynie
        aby zarejestrować maszynę i rozpocząć generowanie kluczy.`,
    },
    noLicenseBox: `<p>
              <strong>YubiKey module</strong>
            </p>
            <br />
            <p>Jest to moduł enterprise dla YubiKey</p>>
            <p>zarządzania i provisioningu.</p>`,
    list: {
      headers: {
        name: 'Nazwa',
        ip: 'Adres IP',
        status: 'Status',
        actions: 'Akcję',
      },
      editButton: {
        delete: 'Usuń provisionera',
      },
      status: {
        available: 'Dostępny',
        unavailable: 'Niedostępny',
      },
    },
    messages: {
      codeCopied: 'Komenda skopiowana.',
    },
  },
  openidAllow: {
    header: '{name: string} chciałby:',
    scopes: {
      openid: 'Użyć danych z twojego profilu do przyszłych logowań.',
      profile:
        'Poznać podstawowe informacje z twojego profilu, takie jak login, imię itp',
      email: 'Poznać twój adres e-mail.',
      phone: 'Poznać twój numer telefonu.',
    },
		controls: {
			accept: 'Akceptuj',
			cancel: 'Anuluj',
		}
  },
  networkOverview: {
    pageTitle: 'Przegląd sieci',
    controls: {
      editNetwork: 'Edycja ustawień sieci',
      configureNetwork: 'Konfiguracja ustawień sieci',
    },
    filterLabels: {
      grid: 'Widok siatki',
      list: 'Widok listy',
    },
    stats: {
      currentlyActiveUsers: 'Obecnie aktywni użytkownicy',
      currentlyActiveDevices: 'Obecnie aktywne urządzenia',
      activeUsersFilter: 'Aktywni użytkownicy w {hour: number}H',
      activeDevicesFilter: 'Aktywne urządzenia w {hour: number}H',
      totalTransfer: 'Całkowity transfer:',
      activityIn: 'Aktywność w {hour: number}H',
      in: 'Przychodzący:',
      out: 'Wychodzący:',
    },
  },
  connectedUsersOverview: {
    pageTitle: 'Podłączeni użytkownicy',
    noUsersMessage: 'Obecnie nie ma żadnych podłączonych użytkowników',
    userList: {
      username: 'Nazwa użytkownika',
      device: 'Urządzenia:',
      connected: 'Połączony:',
      deviceLocation: 'Lokacja urządzenia',
      networkUsage: 'Użycie sieci',
    },
  },
  networkPage: {
    pageTitle: 'Edycja sieci',
  },
  activityOverview: {
    header: 'Strumien aktywności',
    noData: 'Obecnie nie wykryto żadnej aktywności',
  },
  networkConfiguration: {
    header: 'Konfiguracja sieci',
    form: {
      messages: {
        gateway:
          'Adres publiczny Gatewaya, używany przez użytkowników VPN do łączenia się.',
        dns: 'Określ resolwery DNS, które mają odpytywać, gdy interfejs wireguard jest aktywny.',
        allowedIps:
          'Lista adresów/masek, które powinny być routowane przez sieć VPN.',
        networkModified: 'Sieć zmodyfikowana.',
        networkCreated: 'Sieć utworzona.',
      },
      fields: {
        name: {
          label: 'Nazwa sieci',
        },
        address: {
          label: 'Adres i maska sieci VPN',
        },
        endpoint: {
          label: 'Adres gatewaya',
        },
        allowedIps: {
          label: 'Dozwolone adresy IP',
        },
        port: {
          label: 'Port Gatewaya',
        },
        dns: {
          label: 'DNS',
        },
      },
      controls: {
        submit: 'Zapisz zmiany',
        cancel: 'Cofnij',
      },
    },
  },
  gatewaySetup: {
    header: 'Uruchomienie serwera gateway',
    card: {
      title: 'Komenda uruchamiająca serwer gateway',
    },
    controls: {
      status: 'Sprawdź status połączenia',
    },
    messages: {
      runCommand: `
          <p>
            Proszę użyć poniższej komendy na swoim serwerze gateway. Jeśli nie
            wiesz jak, lub masz jakieś problemy
            <a>odwiedź naszą stronę</a>.
          </p>`,
      noConnection: `<p>Brak połączenia proszę uruchom poniższą komendę.</p>`,
      connected: `<p>Gateway połączony.</p>`,
      statusError: 'Nie udało się uzyskać statusu',
    },
  },
  loginPage: {
    pageTitle: 'Wprowadź swoje dane logowania',
    mfa: {
      controls: {
        useAuthenticator: 'Zamiast tego użyj aplikacji Authenticator',
        useWallet: 'Zamiast tego użyj swojego portfela kryptowalutowego',
        useWebauthn: 'Zamiast tego użyj klucza bezpieczeństwa',
        useRecoveryCode: 'Zamiast tego użyj kodu odzyskiwania',
      },
      totp: {
        header:
          'Użyj kodu z aplikacji uwierzytelniającej i kliknij przycisk, aby kontynuować',
        form: {
          fields: {
            code: {
              placeholder: 'Wprowadź kod uwierzytelniający',
            },
          },
          controls: {
            submit: 'Użyj kodu uwierzytelniającego',
          },
        },
      },
      recoveryCode: {
        header:
          'Wpisz jeden z aktywnych kodów odzyskiwania i kliknij przycisk, aby się zalogować.',
        form: {
          fields: {
            code: {
              placeholder: 'Kod odzyskiwania',
            },
          },
          controls: {
            submit: 'Użyj kodu odzyskiwania',
          },
        },
      },
      wallet: {
        header:
          'Użyj portfela kryptowalutowego, aby się zalogować, proszę podpisać wiadomość w aplikacji portfelowej lub rozszerzeniu.',
        controls: {
          submit: 'Użyj swojego portfela',
        },
        messages: {
          walletError:
            'Portfel został rozłączony podczas procesu podpisywania.',
          walletErrorMfa:
            'Portfel nie jest autoryzowany do logowania MFA. Proszę użyć autoryzowanego portfela.',
        },
      },
      webauthn: {
        header:
          'Gdy jesteś gotowy do uwierzytelnienia, naciśnij przycisk poniżej.',
        controls: {
          submit: 'Użyj klucza bezpieczeństwa',
        },
        messages: {
          error: 'Nie udało się odczytać klucza. Proszę spróbować ponownie.',
        },
      },
    },
  },
};

export default pl;
