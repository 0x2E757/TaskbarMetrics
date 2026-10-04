/// Fluent check box in the tokens of the toggle switch, accent when checked. The
/// mark is the system's hinted Segoe Fluent Icons glyph: a stroked Path blurs into
/// uneven legs at 100 %. Placed into an element's resources, a size restyles only
/// the check boxes inside it.
pub struct CheckBoxStyle {
    /// Side of the box.
    box_size: u32,
    corner: u32,
    glyph: u32,
}

impl CheckBoxStyle {
    pub const PLACEHOLDER: &'static str = "<!--CHECK-BOX-->";
    /// The smaller box of the table toolbar, beside the 15 px title.
    pub const COMPACT_PLACEHOLDER: &'static str = "<!--CHECK-BOX-16-->";
    const REGULAR: Self = Self {
        box_size: 20,
        corner: 4,
        glyph: 12,
    };
    const COMPACT: Self = Self {
        box_size: 16,
        corner: 3,
        glyph: 10,
    };

    fn style(&self) -> String {
        let Self {
            box_size,
            corner,
            glyph,
        } = self;
        format!(
            r#"<Style TargetType="CheckBox"><Setter Property="UseSystemFocusVisuals" Value="True"/><Setter Property="MinWidth" Value="0"/><Setter Property="Template"><Setter.Value><ControlTemplate TargetType="CheckBox">
   <Grid Background="Transparent" ColumnSpacing="8"><Grid.ColumnDefinitions><ColumnDefinition Width="Auto"/><ColumnDefinition Width="Auto"/></Grid.ColumnDefinitions>
    <VisualStateManager.VisualStateGroups><VisualStateGroup x:Name="CombinedStates">
     <VisualState x:Name="UncheckedNormal"/>
     <VisualState x:Name="UncheckedPointerOver"><VisualState.Setters><Setter Target="Box.Background" Value="$ctrlHover$"/></VisualState.Setters></VisualState>
     <VisualState x:Name="UncheckedPressed"><VisualState.Setters><Setter Target="Box.Background" Value="$ctrlPress$"/></VisualState.Setters></VisualState>
     <VisualState x:Name="UncheckedDisabled"><VisualState.Setters><Setter Target="Box.BorderBrush" Value="$disabled$"/><Setter Target="Label.Opacity" Value="0.55"/></VisualState.Setters></VisualState>
     <VisualState x:Name="CheckedNormal"><VisualState.Setters><Setter Target="Box.Background" Value="$accent$"/><Setter Target="Box.BorderBrush" Value="$accent$"/><Setter Target="Mark.Visibility" Value="Visible"/></VisualState.Setters></VisualState>
     <VisualState x:Name="CheckedPointerOver"><VisualState.Setters><Setter Target="Box.Background" Value="$accentHover$"/><Setter Target="Box.BorderBrush" Value="$accentHover$"/><Setter Target="Mark.Visibility" Value="Visible"/></VisualState.Setters></VisualState>
     <VisualState x:Name="CheckedPressed"><VisualState.Setters><Setter Target="Box.Background" Value="$accentPress$"/><Setter Target="Box.BorderBrush" Value="$accentPress$"/><Setter Target="Mark.Visibility" Value="Visible"/></VisualState.Setters></VisualState>
     <VisualState x:Name="CheckedDisabled"><VisualState.Setters><Setter Target="Box.Background" Value="$disabled$"/><Setter Target="Box.BorderBrush" Value="$disabled$"/><Setter Target="Mark.Visibility" Value="Visible"/><Setter Target="Label.Opacity" Value="0.55"/></VisualState.Setters></VisualState>
    </VisualStateGroup></VisualStateManager.VisualStateGroups>
    <Border x:Name="Box" Width="{box_size}" Height="{box_size}" CornerRadius="{corner}" BorderThickness="1" BorderBrush="$text2$" Background="Transparent" VerticalAlignment="Center" Control.IsTemplateFocusTarget="True">
     <FontIcon x:Name="Mark" FontFamily="Segoe Fluent Icons" Glyph="&#xE73E;" FontSize="{glyph}" Foreground="$onAccent$" HorizontalAlignment="Center" VerticalAlignment="Center" Visibility="Collapsed"/>
    </Border>
    <ContentPresenter x:Name="Label" Grid.Column="1" Content="{{TemplateBinding Content}}" VerticalAlignment="Center"/>
   </Grid></ControlTemplate></Setter.Value></Setter></Style>"#
        )
    }

    /// `markup` with the placeholders replaced by the styles.
    pub fn markup(markup: &str) -> String {
        markup
            .replace(Self::PLACEHOLDER, &Self::REGULAR.style())
            .replace(Self::COMPACT_PLACEHOLDER, &Self::COMPACT.style())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_placeholder_gets_its_box_size() {
        let markup = CheckBoxStyle::markup("<!--CHECK-BOX-->|<!--CHECK-BOX-16-->");
        let (regular, compact) = markup.split_once('|').unwrap();
        assert!(regular.contains(r#"Width="20" Height="20" CornerRadius="4""#));
        assert!(compact.contains(r#"Width="16" Height="16" CornerRadius="3""#));
        assert!(
            compact.contains(r#"FontSize="10""#) && regular.contains("{TemplateBinding Content}")
        );
    }
}
