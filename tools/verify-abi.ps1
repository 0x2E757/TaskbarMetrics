param([string]$Sdk = 'C:\Program Files (x86)\Windows Kits\10\Include\10.0.26100.0')
$ErrorActionPreference = 'Stop'
# Compare the manually used GUIDs / slots against installed SDK declarations.
$spec = @(
    @('windows.ui.xaml.controls.primitives.h', 'IButtonBase', 'fa002c1a-494e-46cf-91d4-e14a8d798674', @{add_Click=14; remove_Click=15}),
    @('windows.ui.xaml.h', 'IRoutedEventHandler', 'a856e674-b0b6-4bc3-bba8-1ba06e40d4b5', @{Invoke=3}),
    @('windows.ui.xaml.input.h', 'IPointerEventHandler', 'e4385929-c004-4bcf-8970-359486e39f88', @{Invoke=3}),
    @('windows.ui.xaml.input.h', 'IPointerRoutedEventArgs', 'da628f0a-9752-49e2-bde2-49eccab9194d', @{GetCurrentPoint=10}),
    @('windows.ui.input.h', 'IPointerPoint', 'e995317d-7296-42d9-8233-c5be73b74a4a', @{get_Position=7}),
    @('windows.ui.xaml.controls.h', 'IToggleSwitch', '331d8f00-c5f9-46a5-b6c8-ede539304567', @{get_IsOn=6; put_IsOn=7}),
    @('windows.ui.xaml.media.animation.h', 'IStoryboard', 'd45c1e6e-3594-460e-981a-32271bd3aa06', @{Stop=8; Begin=9}),
    @('windows.ui.xaml.controls.primitives.h', 'IFlyoutBase', '723eea0b-d12e-430d-a9f0-9bb32bbf9913', @{ShowAt=14; Hide=15}),
    @('windows.ui.xaml.controls.h', 'IColorPicker', '6232e371-5c64-43cb-8b35-7f82dde36740', @{get_Color=6; put_Color=7}),
    @('windows.ui.xaml.media.h', 'ISolidColorBrush', '9d850850-66f3-48df-9a8f-824bd5e070af', @{get_Color=6; put_Color=7}),
    @('windows.ui.xaml.controls.primitives.h', 'IRangeBase', 'fa002c1a-494e-46cf-91d4-e14a8d798675', @{get_Value=14; put_Value=15}),
    @('windows.ui.xaml.controls.h', 'ITextBox', 'e48f5a8b-1dff-4352-a1f4-e516514ec882', @{get_Text=6; put_Text=7}),
    @('windows.ui.xaml.hosting.h', 'IElementCompositionPreviewStatics', '08c92b38-ec99-4c55-bc85-a1c180b27646', @{GetElementVisual=6}),
    @('windows.ui.composition.h', 'ICompositionObject', 'bcb4ad45-7609-4550-934f-16002a68fded', @{get_Compositor=6}),
    @('windows.ui.composition.h', 'ICompositor5', '48ea31ad-7fcd-4076-a79c-90cc4b852c9b', @{CreateRoundedRectangleGeometry=20}),
    @('windows.ui.composition.h', 'ICompositor6', '7a38b2bd-cec8-4eeb-830f-d8d07aedebc3', @{CreateGeometricClipWithGeometry=7}),
    @('windows.ui.composition.h', 'ICompositionRoundedRectangleGeometry', '8770c822-1d50-4b8b-b013-7c9a0e46935f', @{put_CornerRadius=7; put_Size=11}),
    @('windows.ui.composition.h', 'ICompositionGeometry', 'e985217c-6a17-4207-abd8-5fd3dd612a9d', @{}),
    @('windows.ui.composition.h', 'ICompositionClip', '1ccd2a52-cfc7-4ace-9983-146bb8eb6a3c', @{}),
    @('windows.ui.composition.h', 'IVisual', '117e202d-a859-4c89-873b-c2aa566788e3', @{put_Clip=15}),
    @('windows.ui.xaml.shapes.h', 'IPolyline', '91dc62f8-42b3-47f3-8476-c55124a7c4c6', @{get_Points=8}),
    @('windows.ui.xaml.shapes.h', 'IPolygon', 'e3755c19-2e4d-4bcc-8d34-86871957fa01', @{get_Points=8}),
    @('windows.ui.xaml.media.h', 'ILinearGradientBrush', '8e96d16b-bb84-4c6f-9dbf-9d6c5c6d9c39', @{put_StartPoint=7; put_EndPoint=9}),
    @('windows.ui.xaml.h', 'IFrameworkElement', 'a391d09b-4a99-4b7c-9d8d-6fa5d01f6fbf', @{get_Resources=7; get_ActualWidth=13; get_ActualHeight=14; put_Width=16; put_Height=18; put_HorizontalAlignment=28; put_VerticalAlignment=30; put_Margin=32; get_Name=33; put_Name=34}),
    @('windows.ui.xaml.h', 'IResourceDictionary', 'c1ea4f24-d6de-4191-8e3a-f48601f7489c', @{get_MergedDictionaries=8; get_ThemeDictionaries=9}),
    @('windows.foundation.h', 'IPropertyValueStatics', '629bdbc8-d932-4ff4-96b9-8d96c5c1e858', @{CreateString=18}),
    @('WeakReference.h', 'IWeakReferenceSource', '00000038-0000-0000-c000-000000000046', @{GetWeakReference=3}),
    @('WeakReference.h', 'IWeakReference', '00000037-0000-0000-c000-000000000046', @{Resolve=3}),
    @('windows.ui.xaml.markup.h', 'IXamlReaderStatics', '9891c6bd-534f-4955-b85a-8a8dc0dca602', @{Load=6}),
    @('windows.ui.xaml.controls.h', 'IContentControl', 'a26dd1dc-cd44-435c-be94-01d6241c231c', @{put_Content=7}),
    @('windows.ui.xaml.controls.h', 'IControl', 'a8912263-2951-4f58-a9c5-5a134eaa7f07', @{ApplyTemplate=45}),
    @('windows.ui.xaml.controls.h', 'IBorder', '797c4539-45bd-4633-a044-bfb02ef5170f', @{get_BorderThickness=8; put_BorderThickness=9; get_CornerRadius=12; put_CornerRadius=13}),
    @('windows.ui.xaml.h', 'IDependencyObject', '5c526665-f60e-4912-af59-5fe0680f089d', @{}),
    @('windows.ui.xaml.h', 'IUIElement', '676d0be9-b65c-41c6-ba40-58cf87f201c1', @{put_Opacity=10; put_IsHitTestVisible=20; get_Visibility=21; TransformToVisual=98}),
    @('windows.ui.xaml.controls.h', 'IPanel', 'a50a4bbd-8361-469c-90da-e9a40c7474df', @{get_Children=6}),
    @('windows.ui.xaml.controls.h', 'ITextBlock', 'ae2d9271-3b4a-45fc-8468-f7949548f4d5', @{put_Text=27}),
    @('windows.ui.xaml.controls.h', 'IGrid', 'fd104460-2e15-4ba3-8b8f-fa693a4161e9', @{get_RowDefinitions=6; get_ColumnDefinitions=7}),
    @('windows.ui.xaml.controls.h', 'IGridStatics', '64fe2e9f-f951-42b6-a9ce-bb179af11595', @{SetRowSpan=14; SetColumnSpan=17}),
    @('windows.ui.xaml.media.h', 'IVisualTreeHelperStatics', 'e75758c4-d25d-4b1d-971f-596f17f12baa', @{GetChild=10; GetChildrenCount=11; GetParent=12}),
    @('windows.ui.xaml.media.h', 'IGeneralTransform', 'a06798b7-a2ec-415f-ade2-eade9333f2c7', @{TransformBounds=9}),
    @('windows.system.h', 'IDispatcherQueue', '603e88e4-a338-4ffe-a457-a5cfb9ceb899', @{TryEnqueue=7}),
    @('windows.system.h', 'IDispatcherQueueStatics', 'a96d83d7-9371-4517-9245-d0824ac12c74', @{GetForCurrentThread=6}),
    @('windows.system.h', 'IDispatcherQueueHandler', 'dfa2dc9c-1a2d-4917-98f2-939af1d6e0c8', @{Invoke=3}),
    @('xamlom.h', 'IXamlDiagnostics', '18c9e2b6-3f43-4116-9f2b-ff935d7770d2', @{GetIInspectableFromHandle=6}),
    @('xamlom.h', 'IVisualTreeService', 'a593b11a-d17f-48bb-8f66-83910731c8a5', @{AdviseVisualTreeChange=3; UnadviseVisualTreeChange=4})
)
$cache = @{}
foreach ($item in $spec) {
    $file, $name, $guid, $slots = $item
    if (-not $cache.ContainsKey($file)) {
        $folder = if ($file -eq 'xamlom.h') { 'um' } else { 'winrt' }
        $cache[$file] = [IO.File]::ReadAllText((Join-Path $Sdk "$folder\$file"))
    }
    $pattern = 'MIDL_INTERFACE\("([^"]+)"\)\s*' + $name + '\s*:\s*public (IInspectable|IUnknown)\s*\{(.*?)\n\s*\};'
    $match = [regex]::Match($cache[$file], $pattern, 'Singleline')
    if (-not $match.Success -or $match.Groups[1].Value -ne $guid) { throw "GUID mismatch: $name" }
    $index = if ($match.Groups[2].Value -eq 'IInspectable') { 6 } else { 3 }
    $actual = @{}
    foreach ($method in [regex]::Matches($match.Groups[3].Value, 'virtual\s+HRESULT\s+(?:STDMETHODCALLTYPE\s+)?(\w+)\s*\((.*?)\)\s*=\s*0', 'Singleline')) {
        $actual[$method.Groups[1].Value] = $index
        $index++
    }
    foreach ($method in $slots.Keys) {
        if ($actual[$method] -ne $slots[$method]) { throw "Slot mismatch: $name.$method" }
    }
}
$xaml = $cache['windows.ui.xaml.h']
if ($xaml -notmatch 'uuid\("f5f69427-55ed-5512-8429-d4f6626dfcdd"\)\)\s*IMap<IInspectable\*, IInspectable\*>') {
    throw 'GUID mismatch: IMap<Object,Object>'
}
$map = [regex]::Match($xaml, 'typedef struct __FIMap_2_IInspectable_IInspectableVtbl\s*\{(.*?)END_INTERFACE', 'Singleline')
$methods = @([regex]::Matches($map.Groups[1].Value, 'STDMETHODCALLTYPE\* (\w+)') | ForEach-Object { $_.Groups[1].Value })
foreach ($slot in @{Lookup=6; HasKey=8; Insert=10}.GetEnumerator()) {
    if ($methods[$slot.Value] -ne $slot.Key) { throw "Slot mismatch: IMap.$($slot.Key)" }
}
Write-Output "SDK ABI verified: $($spec.Count + 1) interface GUIDs and all selected method slots."
