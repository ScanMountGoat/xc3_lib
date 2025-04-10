const int undef = 0;
layout(binding = 0, std140) uniform _support_buffer {
    uint alpha_test;
    uint is_bgra[8];
    precise vec4 viewport_inverse;
    precise vec4 viewport_size;
    int frag_scale_count;
    precise float render_scale[73];
    ivec4 tfe_offset;
    int tfe_vertex_count;
}support_buffer;
layout(binding = 5, std140) uniform _U_Mate {
    vec4 gWrkFl4[2];
    vec4 gWrkCol;
    vec4 gMatCol;
}U_Mate;
layout(binding = 2, std140) uniform _fp_c1 {
    precise vec4 data[4096];
}fp_c1;
layout(location = 0) in vec4 in_attr0;
layout(location = 1) in vec4 in_attr1;
layout(location = 2) in vec4 in_attr2;
layout(location = 3) in vec4 in_attr3;
layout(location = 0) out vec4 out_attr0;
layout(location = 1) out vec4 out_attr1;
layout(location = 2) out vec4 out_attr2;
layout(location = 3) out vec4 out_attr3;
layout(location = 4) out vec4 out_attr4;
void main() {
    precise float temp_0;
    precise float temp_1;
    precise float temp_2;
    precise float temp_3;
    precise float temp_4;
    precise float temp_5;
    precise float temp_6;
    precise float temp_7;
    precise float temp_8;
    precise float temp_9;
    precise float temp_10;
    precise float temp_11;
    precise float temp_12;
    precise float temp_13;
    precise float temp_14;
    precise float temp_15;
    precise float temp_16;
    precise float temp_17;
    precise float temp_18;
    precise float temp_19;
    precise float temp_20;
    precise float temp_21;
    precise float temp_22;
    precise float temp_23;
    precise float temp_24;
    precise float temp_25;
    precise float temp_26;
    precise float temp_27;
    precise float temp_28;
    precise float temp_29;
    precise float temp_30;
    precise float temp_31;
    precise float temp_32;
    precise float temp_33;
    precise float temp_34;
    precise float temp_35;
    precise float temp_36;
    precise float temp_37;
    precise float temp_38;
    precise float temp_39;
    precise float temp_40;
    precise float temp_41;
    precise float temp_42;
    precise float temp_43;
    precise float temp_44;
    precise float temp_45;
    precise float temp_46;
    precise float temp_47;
    precise float temp_48;
    precise float temp_49;
    precise float temp_50;
    precise float temp_51;
    precise float temp_52;
    precise float temp_53;
    precise float temp_54;
    precise float temp_55;
    precise float temp_56;
    precise float temp_57;
    precise float temp_58;
    precise float temp_59;
    precise float temp_60;
    uint temp_61;
    precise float temp_62;
    precise float temp_63;
    precise float temp_64;
    precise float temp_65;
    bool temp_66;
    precise float temp_67;
    precise float temp_68;
    precise float temp_69;
    precise float temp_70;
    precise float temp_71;
    bool temp_72;
    precise float temp_73;
    precise float temp_74;
    precise float temp_75;
    precise float temp_76;
    precise float temp_77;
    precise float temp_78;
    precise float temp_79;
    precise float temp_80;
    precise float temp_81;
    precise float temp_82;
    precise float temp_83;
    int temp_84;
    precise float temp_85;
    precise float temp_86;
    precise float temp_87;
    precise float temp_88;
    precise float temp_89;
    precise float temp_90;
    precise float temp_91;
    precise float temp_92;
    precise float temp_93;
    precise float temp_94;
    precise float temp_95;
    precise float temp_96;
    precise float temp_97;
    precise float temp_98;
    precise float temp_99;
    precise float temp_100;
    precise float temp_101;
    precise float temp_102;
    precise float temp_103;
    precise float temp_104;
    precise float temp_105;
    precise float temp_106;
    temp_0 = in_attr0.x;
    temp_1 = in_attr0.y;
    temp_2 = in_attr0.z;
    temp_3 = in_attr3.w;
    temp_4 = in_attr2.w;
    temp_5 = in_attr3.y;
    temp_6 = in_attr2.y;
    temp_7 = in_attr3.x;
    temp_8 = in_attr2.x;
    temp_9 = temp_0 * temp_0;
    temp_10 = 1. / temp_3;
    temp_11 = fma(temp_1, temp_1, temp_9);
    temp_12 = 1. / temp_4;
    temp_13 = fma(temp_2, temp_2, temp_11);
    temp_14 = temp_5 * temp_10;
    temp_15 = inversesqrt(temp_13);
    temp_16 = temp_7 * temp_10;
    temp_17 = 0. - temp_14;
    temp_18 = fma(temp_12, temp_6, temp_17);
    temp_19 = 0. - temp_16;
    temp_20 = fma(temp_12, temp_8, temp_19);
    temp_21 = temp_18 * 0.5;
    temp_22 = temp_0 * temp_15;
    temp_23 = temp_1 * temp_15;
    temp_24 = temp_2 * temp_15;
    temp_25 = temp_20 * 0.5;
    temp_26 = abs(temp_25);
    temp_27 = abs(temp_21);
    temp_28 = max(temp_26, temp_27);
    temp_29 = dFdy(temp_22);
    temp_30 = dFdy(temp_23);
    temp_31 = 1. * temp_29;
    temp_32 = in_attr1.y;
    temp_33 = dFdx(temp_22);
    temp_34 = in_attr2.z;
    temp_35 = 1. * temp_30;
    temp_36 = in_attr1.x;
    temp_37 = dFdy(temp_24);
    temp_38 = temp_33 * temp_33;
    temp_39 = temp_31 * temp_31;
    temp_40 = 1. * temp_37;
    temp_41 = dFdx(temp_23);
    temp_42 = fma(temp_35, temp_35, temp_39);
    temp_43 = max(temp_28, 1.);
    temp_44 = 0.01 * U_Mate.gMatCol.x;
    temp_45 = 1. / temp_43;
    temp_46 = dFdx(temp_24);
    temp_47 = fma(temp_41, temp_41, temp_38);
    temp_48 = fma(temp_40, temp_40, temp_42);
    temp_49 = fma(0.01, U_Mate.gMatCol.y, temp_44);
    temp_50 = fma(temp_46, temp_46, temp_47);
    temp_51 = temp_32 + 0.004;
    temp_52 = clamp(temp_51, 0., 1.);
    temp_53 = temp_34 * 8.;
    temp_54 = floor(temp_53);
    temp_55 = fma(0.01, U_Mate.gMatCol.z, temp_49);
    temp_56 = temp_48 + temp_50;
    temp_57 = temp_52 * 3.;
    temp_58 = temp_21 * temp_45;
    temp_59 = trunc(temp_57);
    temp_60 = max(temp_59, 0.);
    temp_61 = uint(temp_60);
    temp_62 = temp_25 * temp_45;
    temp_63 = temp_56 * 0.5;
    temp_64 = 0. - U_Mate.gMatCol.x;
    temp_65 = temp_55 + temp_64;
    temp_66 = temp_58 >= 0.;
    temp_67 = temp_66 ? 1. : 0.;
    temp_68 = abs(temp_58);
    temp_69 = inversesqrt(temp_68);
    temp_70 = 0. - U_Mate.gWrkFl4[0].z;
    temp_71 = temp_70 + 1.;
    temp_72 = temp_62 >= 0.;
    temp_73 = temp_72 ? 1. : 0.;
    temp_74 = abs(temp_62);
    temp_75 = inversesqrt(temp_74);
    temp_76 = temp_54 * 0.003921569;
    temp_77 = min(temp_63, 0.18);
    temp_78 = floor(temp_76);
    temp_79 = temp_67 * 0.6666667;
    temp_80 = 1. / temp_69;
    temp_81 = fma(temp_71, temp_71, temp_77);
    temp_82 = clamp(temp_81, 0., 1.);
    temp_83 = 1. / temp_75;
    temp_84 = int(temp_61) << 6;
    temp_85 = sqrt(temp_82);
    temp_86 = 0. - temp_54;
    temp_87 = temp_53 + temp_86;
    temp_88 = float(uint(temp_84));
    temp_89 = 0. - U_Mate.gMatCol.y;
    temp_90 = temp_55 + temp_89;
    temp_91 = fma(temp_73, 0.33333334, temp_79);
    temp_92 = 0. - U_Mate.gMatCol.z;
    temp_93 = temp_55 + temp_92;
    temp_94 = fma(temp_65, U_Mate.gWrkFl4[1].y, U_Mate.gMatCol.x);
    temp_95 = 0. - temp_78;
    temp_96 = temp_76 + temp_95;
    temp_97 = fma(temp_90, U_Mate.gWrkFl4[1].y, U_Mate.gMatCol.y);
    temp_98 = fma(temp_93, U_Mate.gWrkFl4[1].y, U_Mate.gMatCol.z);
    temp_99 = fma(temp_22, 0.5, 0.5);
    temp_100 = fma(temp_23, 0.5, 0.5);
    temp_101 = fma(temp_24, 1000., 0.5);
    temp_102 = temp_78 * 0.003921569;
    temp_103 = temp_88 * 0.003921569;
    temp_104 = temp_91 + 0.01;
    temp_105 = 0. - temp_85;
    temp_106 = temp_105 + 1.;
    out_attr0.x = temp_94;
    out_attr0.y = temp_97;
    out_attr0.z = temp_98;
    out_attr0.w = temp_103;
    out_attr1.x = U_Mate.gWrkFl4[0].w;
    out_attr1.y = temp_106;
    out_attr1.z = U_Mate.gWrkFl4[0].x;
    out_attr1.w = 0.008235293;
    out_attr2.x = temp_99;
    out_attr2.y = temp_100;
    out_attr2.z = 1.;
    out_attr2.w = temp_101;
    out_attr3.x = temp_83;
    out_attr3.y = temp_80;
    out_attr3.z = 0.;
    out_attr3.w = temp_104;
    out_attr4.x = temp_87;
    out_attr4.y = temp_96;
    out_attr4.z = temp_102;
    out_attr4.w = temp_36;
    return;
}
