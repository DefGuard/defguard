{% extends "mail_base" %}
{% block content %}
  <div style="background-color:#f9f9f9;">
    <!--[if mso | IE]><table align="center" border="0" cellpadding="0" cellspacing="0" class="" role="presentation" style="width:600px;" width="600" bgcolor="#f9f9f9" ><tr><td style="line-height:0px;font-size:0px;mso-line-height-rule:exactly;"><![endif]-->
    <!--[if mso | IE]></td></tr></table><table align="center" border="0" cellpadding="0" cellspacing="0" class="" role="presentation" style="width:600px;" width="600" bgcolor="#f9f9f9" ><tr><td style="line-height:0px;font-size:0px;mso-line-height-rule:exactly;"><![endif]-->
    <div style="background:#f9f9f9;background-color:#f9f9f9;margin:0px auto;max-width:600px;">
      <table align="center" border="0" cellpadding="0" cellspacing="0" role="presentation" style="background:#f9f9f9;background-color:#f9f9f9;width:100%;">
        <tbody>
          <tr>
            <td style="direction:ltr;font-size:0px;padding:20px 0;text-align:center;">
              <!--[if mso | IE]><table role="presentation" border="0" cellpadding="0" cellspacing="0"><tr><td class="" style="vertical-align:top;width:600px;" ><![endif]-->
              <div class="mj-column-per-100 mj-outlook-group-fix" style="font-size:0px;text-align:left;direction:ltr;display:inline-block;vertical-align:top;width:100%;">
                <table border="0" cellpadding="0" cellspacing="0" role="presentation" style="vertical-align:top;" width="100%">
                  <tbody>
                    <tr>
                      <td align="center" style="font-size:0px;padding:10px 25px;word-break:break-word;">
                        <div style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#000000;">
                          <p>You're receiving this email because a new account has been created for you.</p>
                        </div>
                      </td>
                    </tr>
                    <tr>
                      <td align="center" vertical-align="middle" style="font-size:0px;padding:10px 25px;word-break:break-word;">
                        <table border="0" cellpadding="0" cellspacing="0" role="presentation" style="border-collapse:separate;width:140px;line-height:100%;">
                          <tbody>
                            <tr>
                              <td align="center" bgcolor="#OC8CE0" role="presentation" style="border:none;border-radius:10px;cursor:auto;height:50px;mso-padding-alt:10px 25px;background:#OC8CE0;" valign="middle">
                                <a href="{{url}}" style="display:inline-block;width:90px;background:#OC8CE0;color:#FFFFFF !important;font-family:Roboto;font-size:15px;font-weight:600;line-height:normal;margin:0;text-decoration:none;text-transform:none;padding:10px 25px;mso-padding-alt:0px;border-radius:10px;" target="_blank"> Click here </a>
                              </td>
                            </tr>
                          </tbody>
                        </table>
                      </td>
                    </tr>
                    <tr>
                      <td align="center" style="font-size:0px;padding:10px 25px;word-break:break-word;">
                        <div style="font-family:Roboto;font-size:12px;line-height:1;text-align:center;color:#899CA8;">
                          <p>If the provided button doesnt work, please copy & paste the following url in your browser: </p>
                          <a style="color:#899CA8;" href="{{url}}" target="_blank">{{ url }}</a>
                        </div>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <!--[if mso | IE]></td></tr></table><![endif]-->
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <!--[if mso | IE]></td></tr></table><![endif]-->
  </div>
{% endblock content %}
