/// Button template without the pressed tilt (`PointerDownThemeAnimation`) of the
/// system style: its scale transform blurs the text while the button is held.
/// States use the same lightweight-styling brushes (`ButtonBackgroundPointerOver`, …),
/// so resources overridden on a button (the accent «Back to live») still apply.
pub struct PlainButton;
impl PlainButton {
    pub const PLACEHOLDER: &'static str = "<!--PLAIN-BUTTON-->";
    const TEMPLATE: &'static str = r#"<ControlTemplate x:Key="PlainButton" TargetType="Button"><ContentPresenter x:Name="ContentPresenter" Background="{TemplateBinding Background}" BorderBrush="{TemplateBinding BorderBrush}" BorderThickness="{TemplateBinding BorderThickness}" CornerRadius="{TemplateBinding CornerRadius}" Padding="{TemplateBinding Padding}" Foreground="{TemplateBinding Foreground}" Content="{TemplateBinding Content}" ContentTemplate="{TemplateBinding ContentTemplate}" ContentTransitions="{TemplateBinding ContentTransitions}" HorizontalContentAlignment="{TemplateBinding HorizontalContentAlignment}" VerticalContentAlignment="{TemplateBinding VerticalContentAlignment}" AutomationProperties.AccessibilityView="Raw">
  <VisualStateManager.VisualStateGroups><VisualStateGroup x:Name="CommonStates">
   <VisualState x:Name="Normal"/>
   <VisualState x:Name="PointerOver"><VisualState.Setters><Setter Target="ContentPresenter.Background" Value="{ThemeResource ButtonBackgroundPointerOver}"/><Setter Target="ContentPresenter.BorderBrush" Value="{ThemeResource ButtonBorderBrushPointerOver}"/><Setter Target="ContentPresenter.Foreground" Value="{ThemeResource ButtonForegroundPointerOver}"/></VisualState.Setters></VisualState>
   <VisualState x:Name="Pressed"><VisualState.Setters><Setter Target="ContentPresenter.Background" Value="{ThemeResource ButtonBackgroundPressed}"/><Setter Target="ContentPresenter.BorderBrush" Value="{ThemeResource ButtonBorderBrushPressed}"/><Setter Target="ContentPresenter.Foreground" Value="{ThemeResource ButtonForegroundPressed}"/></VisualState.Setters></VisualState>
   <VisualState x:Name="Disabled"><VisualState.Setters><Setter Target="ContentPresenter.Background" Value="{ThemeResource ButtonBackgroundDisabled}"/><Setter Target="ContentPresenter.BorderBrush" Value="{ThemeResource ButtonBorderBrushDisabled}"/><Setter Target="ContentPresenter.Foreground" Value="{ThemeResource ButtonForegroundDisabled}"/></VisualState.Setters></VisualState>
  </VisualStateGroup></VisualStateManager.VisualStateGroups>
 </ContentPresenter></ControlTemplate>"#;
    /// `markup` with the placeholder replaced by the template resource.
    pub fn markup(markup: &str) -> String {
        markup.replace(Self::PLACEHOLDER, Self::TEMPLATE)
    }
}
