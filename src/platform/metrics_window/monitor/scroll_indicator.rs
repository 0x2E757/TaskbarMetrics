/// Always visible 3 px scroll indicator of the artboards (`text3` at 45 %, 3 px
/// from the edge) instead of the system bar that appears only on pointer input.
/// Placed into a ScrollViewer's resources, it restyles only that viewer's bars.
pub struct ScrollIndicator;
impl ScrollIndicator {
    pub const PLACEHOLDER: &'static str = "<!--SCROLL-INDICATOR-->";
    const STYLE: &'static str = r#"<Style TargetType="ScrollBar"><Setter Property="IsTabStop" Value="False"/><Setter Property="Template"><Setter.Value><ControlTemplate TargetType="ScrollBar"><Grid Background="Transparent">
  <Grid x:Name="VerticalRoot" Width="9"><Grid.RowDefinitions><RowDefinition Height="Auto"/><RowDefinition Height="Auto"/><RowDefinition Height="Auto"/><RowDefinition/><RowDefinition Height="Auto"/></Grid.RowDefinitions>
   <RepeatButton x:Name="VerticalLargeDecrease" Grid.Row="1" IsTabStop="False" Interval="50"><RepeatButton.Template><ControlTemplate TargetType="RepeatButton"><Grid Background="Transparent"/></ControlTemplate></RepeatButton.Template></RepeatButton>
   <Thumb x:Name="VerticalThumb" Grid.Row="2" MinHeight="24" IsTabStop="False"><Thumb.Template><ControlTemplate TargetType="Thumb"><Grid Background="Transparent"><Border Width="3" HorizontalAlignment="Right" CornerRadius="2" Background="$scrollThumb$"/></Grid></ControlTemplate></Thumb.Template></Thumb>
   <RepeatButton x:Name="VerticalLargeIncrease" Grid.Row="3" IsTabStop="False" Interval="50"><RepeatButton.Template><ControlTemplate TargetType="RepeatButton"><Grid Background="Transparent"/></ControlTemplate></RepeatButton.Template></RepeatButton>
  </Grid>
  <Grid x:Name="HorizontalRoot" Visibility="Collapsed"/>
 </Grid></ControlTemplate></Setter.Value></Setter></Style>"#;
    /// `markup` with the placeholder replaced by the style.
    pub fn markup(markup: &str) -> String {
        markup.replace(Self::PLACEHOLDER, Self::STYLE)
    }
}
